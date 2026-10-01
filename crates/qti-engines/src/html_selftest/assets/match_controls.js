<script>
(function() {
  const container = document.getElementById('question_html_{{CRC}}');
  const choices = Array.from(container.querySelectorAll('.qti-match-choice'));
  const slots = Array.from(container.querySelectorAll('.qti-match-slot'));
  const status = container.querySelector('[role=status]');
  let selected = null;
  function select(choice) {
    selected = choice;
    choices.forEach(button => {
      button.setAttribute('aria-pressed', button === selected ? 'true' : 'false');
      button.classList.toggle('qti-selected', button === selected);
    });
  }
  function updateBank() {
    choices.forEach(choice => {
      choice.disabled = slots.some(slot => slot.dataset.value === choice.dataset.value);
      choice.draggable = !choice.disabled;
    });
  }
  function emptySlot(slot) {
    delete slot.dataset.value;
    slot.className = 'qti-match-slot';
    slot.textContent = 'Drop Your Choice Here';
    slot.removeAttribute('title');
    slot.setAttribute('aria-label', 'Assign a choice to prompt ' + slot.dataset.prompt);
  }
  function assign(choice, slot) {
    if (choice.disabled) return;
    const text = choice.innerText.trim().replace(/\s+/g, ' ');
    slot.textContent = text.length > 30 ? text.substring(0, 27) + '...' : text;
    slot.title = text;
    slot.dataset.value = choice.dataset.value;
    slot.className = 'qti-match-slot ' + Array.from(choice.classList).find(name => name.startsWith('qti-choice-'));
    slot.setAttribute('aria-label', 'Prompt ' + slot.dataset.prompt + ': ' + text + '. Replace with selected choice');
    select(null);
    updateBank();
    clearFeedback_{{CRC}}();
    slot.focus();
    status.textContent = 'Assigned ' + text + ' to prompt ' + slot.dataset.prompt + '.';
  }
  container.qtiBindDrag({
    sources: '.qti-match-choice', targets: '.qti-match-slot', reorder: false, drop: assign
  });
  container.addEventListener('click', event => {
    const choice = event.target.closest('.qti-match-choice');
    if (choice && !choice.disabled) {
      select(choice === selected ? null : choice);
      choice.focus();
      status.textContent = selected ? 'Selected ' + selected.querySelector('.qti-choice-content').textContent.trim() + '. Choose a prompt.' : 'Selection cleared.';
      return;
    }
    const slot = event.target.closest('.qti-match-slot');
    if (!slot) return;
    if (!selected) { status.textContent = 'Select a choice first.'; return; }
    assign(selected, slot);
  });
  container.addEventListener('keydown', event => {
    if (event.key === 'Escape') { select(null); status.textContent = 'Selection cleared.'; }
  });
  container.qtiResetGame = () => {
    container.qtiCancelDrag();
    slots.forEach(emptySlot);
    select(null);
    updateBank();
    status.textContent = 'Matches reset.';
  };
  updateBank();
})();
</script>
