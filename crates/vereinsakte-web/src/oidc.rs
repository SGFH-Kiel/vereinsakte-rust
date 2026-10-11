//! Anmeldung über Microsoft Entra ID (OpenID Connect, Authorization-Code-Flow
//! mit PKCE). Die Web-Schicht spricht nur mit [`LoginProvider`]; Tests nutzen
//! eine Fake-Implementierung.

use async_trait::async_trait;
use openidconnect::core::{
    CoreAuthDisplay, CoreAuthPrompt, CoreAuthenticationFlow, CoreErrorResponseType,
    CoreGenderClaim, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm, CoreJwsSigningAlgorithm,
    CoreProviderMetadata, CoreRevocableToken, CoreRevocationErrorResponse,
    CoreTokenIntrospectionResponse, CoreTokenType,
};
use openidconnect::{
    AdditionalClaims, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    EmptyExtraTokenFields, EndpointMaybeSet, EndpointNotSet, EndpointSet, IdTokenFields, IssuerUrl,
    Nonce, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, StandardErrorResponse,
    StandardTokenResponse, TokenResponse,
};
use serde::{Deserialize, Serialize};
use url::Url;

/// Beginn einer Anmeldung: Ziel beim Anbieter und die in der Sitzung zu
/// merkenden Werte.
#[derive(Clone, Debug)]
pub struct AuthRequest {
    pub url: Url,
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
}

/// Geprüfte Angaben aus dem ID-Token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedLogin {
    pub issuer: String,
    /// Mandant (`tid`).
    pub tenant_id: Option<String>,
    /// Unveränderliche Objekt-ID des Kontos (`oid`).
    pub object_id: Option<String>,
    pub subject: String,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
}

#[async_trait]
pub trait LoginProvider: Send + Sync {
    /// Erzeugt die Weiterleitung zum Anbieter (mit `state`, `nonce` und PKCE).
    fn authorize(&self) -> AuthRequest;

    /// Tauscht den Code gegen Tokens und prüft das ID-Token (Signatur,
    /// Aussteller, Audience, `nonce`).
    async fn finish(
        &self,
        code: &str,
        pkce_verifier: &str,
        nonce: &str,
    ) -> Result<VerifiedLogin, String>;
}

/// Zusätzliche Claims von Entra ID.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct EntraClaims {
    tid: Option<String>,
    oid: Option<String>,
}

impl AdditionalClaims for EntraClaims {}

type EntraIdTokenFields = IdTokenFields<
    EntraClaims,
    EmptyExtraTokenFields,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm,
>;
type EntraTokenResponse = StandardTokenResponse<EntraIdTokenFields, CoreTokenType>;
type EntraClient = Client<
    EntraClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    EntraTokenResponse,
    CoreTokenIntrospectionResponse,
    CoreRevocableToken,
    CoreRevocationErrorResponse,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

/// Anmeldung über die App-Registrierung des Vereins in Microsoft Entra ID.
pub struct EntraProvider {
    client: EntraClient,
    http: reqwest::Client,
}

impl EntraProvider {
    /// Aussteller eines Single-Tenant-Mandanten.
    pub fn issuer(tenant_id: &str) -> String {
        format!("https://login.microsoftonline.com/{tenant_id}/v2.0")
    }

    fn http_client() -> Result<reqwest::Client, String> {
        // Keine Weiterleitungen folgen (Schutz vor SSRF über den Anbieter).
        reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| e.to_string())
    }

    /// Lädt die Metadaten des Mandanten (Discovery) und baut den Client.
    pub async fn discover(
        tenant_id: &str,
        client_id: &str,
        client_secret: &str,
        base_url: &str,
    ) -> Result<Self, String> {
        let http = Self::http_client()?;
        let issuer = IssuerUrl::new(Self::issuer(tenant_id)).map_err(|e| e.to_string())?;
        let metadata = CoreProviderMetadata::discover_async(issuer, &http)
            .await
            .map_err(|e| format!("Discovery bei Microsoft Entra ID fehlgeschlagen: {e}"))?;
        Self::from_metadata(metadata, client_id, client_secret, base_url, http)
    }

    /// Baut den Client aus bereits vorliegenden Metadaten (auch für Tests).
    pub fn from_metadata(
        metadata: CoreProviderMetadata,
        client_id: &str,
        client_secret: &str,
        base_url: &str,
        http: reqwest::Client,
    ) -> Result<Self, String> {
        let redirect =
            RedirectUrl::new(format!("{}/auth/callback", base_url.trim_end_matches('/')))
                .map_err(|e| e.to_string())?;
        let client = EntraClient::from_provider_metadata(
            metadata,
            ClientId::new(client_id.to_string()),
            Some(ClientSecret::new(client_secret.to_string())),
        )
        .set_redirect_uri(redirect);
        Ok(Self { client, http })
    }

    #[cfg(test)]
    pub(crate) fn for_tests(metadata: CoreProviderMetadata) -> Self {
        Self::from_metadata(
            metadata,
            "client-id",
            "secret",
            "http://localhost:8080",
            Self::http_client().unwrap(),
        )
        .unwrap()
    }
}

#[async_trait]
impl LoginProvider for EntraProvider {
    fn authorize(&self) -> AuthRequest {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state, nonce) = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("email".into()))
            .add_scope(Scope::new("profile".into()))
            .set_pkce_challenge(challenge)
            .url();
        AuthRequest {
            url,
            state: state.secret().clone(),
            nonce: nonce.secret().clone(),
            pkce_verifier: verifier.secret().clone(),
        }
    }

    async fn finish(
        &self,
        code: &str,
        pkce_verifier: &str,
        nonce: &str,
    ) -> Result<VerifiedLogin, String> {
        let response = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| e.to_string())?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
            .request_async(&self.http)
            .await
            .map_err(|e| format!("Token-Abruf fehlgeschlagen: {e}"))?;
        let id_token = response.id_token().ok_or("Antwort enthält kein ID-Token")?;
        let claims = id_token
            .claims(
                &self.client.id_token_verifier(),
                &Nonce::new(nonce.to_string()),
            )
            .map_err(|e| format!("ID-Token ungültig: {e}"))?;
        Ok(VerifiedLogin {
            issuer: claims.issuer().to_string(),
            tenant_id: claims.additional_claims().tid.clone(),
            object_id: claims.additional_claims().oid.clone(),
            subject: claims.subject().to_string(),
            email: claims.email().map(|e| e.to_string()),
            preferred_username: claims.preferred_username().map(|u| u.to_string()),
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use openidconnect::core::{CoreResponseType, CoreSubjectIdentifierType};
    use openidconnect::{
        AuthUrl, EmptyAdditionalProviderMetadata, JsonWebKeySet, JsonWebKeySetUrl, ResponseTypes,
        TokenUrl,
    };

    use super::*;

    /// Metadaten eines erfundenen Mandanten ohne Netzwerkzugriff.
    pub fn metadata() -> CoreProviderMetadata {
        let base = "https://login.microsoftonline.com/tenant-1/";
        CoreProviderMetadata::new(
            IssuerUrl::new(EntraProvider::issuer("tenant-1")).unwrap(),
            AuthUrl::new(format!("{base}oauth2/v2.0/authorize")).unwrap(),
            JsonWebKeySetUrl::new(format!("{base}discovery/v2.0/keys")).unwrap(),
            vec![ResponseTypes::new(vec![CoreResponseType::Code])],
            vec![CoreSubjectIdentifierType::Pairwise],
            vec![CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256],
            EmptyAdditionalProviderMetadata {},
        )
        .set_token_endpoint(Some(
            TokenUrl::new(format!("{base}oauth2/v2.0/token")).unwrap(),
        ))
        .set_jwks(JsonWebKeySet::new(vec![]))
    }

    #[test]
    fn authorize_url_uses_pkce_state_and_scopes() {
        let provider = EntraProvider::for_tests(metadata());
        let request = provider.authorize();
        let url = request.url.as_str();
        assert!(
            url.starts_with("https://login.microsoftonline.com/tenant-1/oauth2/v2.0/authorize?"),
            "{url}"
        );
        let query: std::collections::HashMap<_, _> =
            request.url.query_pairs().into_owned().collect();
        assert_eq!(query["state"], request.state);
        assert_eq!(query["nonce"], request.nonce);
        assert_eq!(query["code_challenge_method"], "S256");
        assert!(!query["code_challenge"].is_empty());
        assert_eq!(query["scope"], "openid email profile");
        assert_eq!(query["redirect_uri"], "http://localhost:8080/auth/callback");
        assert_eq!(query["client_id"], "client-id");
    }
}
