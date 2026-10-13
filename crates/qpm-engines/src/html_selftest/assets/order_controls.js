<script>
(function() {
  const container = document.getElementById('question_html_{{CRC}}');
  if (!container || container.qpmOrderInitialized) return;
  container.qpmOrderInitialized = true;
  const list = container.querySelector('.qpm-order-list');
  const initialRows = Array.from(list.children);
  const status = container.querySelector('[role=status]');
  function updatePositions() {
    Array.from(list.children).forEach((row, index) => {
      row.querySelector('.qpm-order-position').textContent = index + 1;
      row.querySelectorAll('.qpm-order-move').forEach(button => {
        const up = button.dataset.direction === 'up';
        button.disabled = up ? index === 0 : index === list.children.length - 1;
        button.setAttribute('aria-label', `Move item ${index + 1} ${up ? 'earlier' : 'later'}`);
      });
    });
  }
  function place(row, before, button) {
    list.insertBefore(row, before);
    updatePositions();
    const focusButton = button.disabled ? row.querySelector('.qpm-order-move:not(:disabled)') : button;
    focusButton.focus();
    clearFeedback_{{CRC}}();
    status.textContent = row.querySelector('.qpm-choice-content').textContent.trim() +
      ' moved to position ' + row.querySelector('.qpm-order-position').textContent + '.';
  }
  function move(button, direction) {
    const row = button.closest('.qpm-order-row');
    const neighbor = direction === 'up' ? row.previousElementSibling : row.nextElementSibling;
    if (neighbor) place(row, direction === 'up' ? neighbor : neighbor.nextElementSibling, button);
  }
  container.qpmBindDrag({
    sources: '.qpm-order-row', targets: '.qpm-order-row', reorder: true,
    drop(row, target, after) {
      place(row, after ? target.nextElementSibling : target, row.querySelector('.qpm-order-move:not(:disabled)'));
    }
  });
  list.addEventListener('click', event => {
    const button = event.target.closest('.qpm-order-move');
    if (button) move(button, button.dataset.direction);
  });
  list.addEventListener('keydown', event => {
    const button = event.target.closest('.qpm-order-move');
    if (!button || !['ArrowUp', 'ArrowDown'].includes(event.key)) return;
    event.preventDefault();
    move(button, event.key === 'ArrowUp' ? 'up' : 'down');
  });
  container.qpmResetGame = () => {
    container.qpmCancelDrag();
    initialRows.forEach(row => list.appendChild(row));
    updatePositions();
    status.textContent = 'Order reset.';
  };
  updatePositions();
})();
</script>
