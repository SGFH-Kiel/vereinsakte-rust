// Aufgabenboard: Karten per Ziehen oder Pfeiltasten in eine andere Spalte legen.
// Ohne dieses Skript funktioniert das Verschieben über das Formular an der Karte.
(() => {
  const board = document.querySelector('[data-board]');
  if (!board) return;
  const token = board.dataset.csrf;
  const columns = Array.from(board.querySelectorAll('[data-status]'));

  const move = async (card, status) => {
    try {
      const body = new URLSearchParams({ status, version: card.dataset.version, ajax: '1' });
      const response = await fetch(`/aufgaben/${card.dataset.id}/status`, {
        method: 'POST',
        headers: { 'X-CSRF-Token': token, 'Content-Type': 'application/x-www-form-urlencoded' },
        body,
      });
      if (!response.ok) {
        window.alert('Die Karte wurde zwischenzeitlich geändert oder darf nicht verschoben werden. Das Board wird neu geladen.');
      }
    } catch (error) {
      window.alert('Das Verschieben ist fehlgeschlagen. Das Board wird neu geladen.');
    }
    window.location.reload();
  };

  board.querySelectorAll('.task-card[data-can-move="1"]').forEach((card) => {
    card.setAttribute('draggable', 'true');
    card.addEventListener('dragstart', (event) => {
      event.dataTransfer.setData('text/plain', card.dataset.id);
      event.dataTransfer.effectAllowed = 'move';
      card.classList.add('dragging');
    });
    card.addEventListener('dragend', () => card.classList.remove('dragging'));
    card.addEventListener('keydown', (event) => {
      if (event.target !== card || !event.altKey) return;
      const index = columns.findIndex((c) => c.contains(card));
      const target = columns[index + (event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0)];
      if (target && target !== columns[index]) {
        event.preventDefault();
        move(card, target.dataset.status);
      }
    });
  });

  columns.forEach((column) => {
    column.addEventListener('dragover', (event) => {
      event.preventDefault();
      column.classList.add('drop-target');
    });
    column.addEventListener('dragleave', () => column.classList.remove('drop-target'));
    column.addEventListener('drop', (event) => {
      event.preventDefault();
      column.classList.remove('drop-target');
      const id = event.dataTransfer.getData('text/plain');
      const card = board.querySelector(`.task-card[data-id="${CSS.escape(id)}"]`);
      if (card && !column.contains(card)) move(card, column.dataset.status);
    });
  });
})();
