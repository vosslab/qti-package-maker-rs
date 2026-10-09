<script>
(function() {
  const container = document.getElementById('question_html_{{CRC}}');
  if (!container || container.qtiMatchInitialized) return;
  container.qtiMatchInitialized = true;
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
  function emptySlot(slot) {
    delete slot.dataset.value;
    slot.className = 'qti-match-slot';
    slot.textContent = 'Drop Your Choice Here';
    slot.removeAttribute('title');
    slot.setAttribute('aria-label', 'Assign a choice to prompt ' + slot.dataset.prompt);
  }
  function assign(choice, slot, move = false) {
    const previous = slot.dataset.value;
    const cleared = move ? slots.filter(other => other !== slot &&
      other.dataset.value === choice.dataset.value) : [];
    cleared.forEach(emptySlot);
    const text = choice.innerText.trim().replace(/\s+/g, ' ');
    slot.textContent = text;
    slot.title = text;
    slot.dataset.value = choice.dataset.value;
    slot.className = 'qti-match-slot ' + Array.from(choice.classList).find(name => name.startsWith('qti-choice-'));
    slot.setAttribute('aria-label', 'Prompt ' + slot.dataset.prompt + ': ' + text + '. Replace with selected choice');
    select(null);
    clearFeedback_{{CRC}}();
    slot.focus();
    let message = cleared.length ? choice.dataset.letter + ' moved from prompt ' +
      cleared.map(other => other.dataset.prompt).join(', ') + ' to prompt ' + slot.dataset.prompt + '.' :
      'Assigned ' + text + ' to prompt ' + slot.dataset.prompt + '.';
    if (previous && previous !== choice.dataset.value) {
      message += ' Replaced ' + choices.find(button => button.dataset.value === previous).dataset.letter + '.';
    }
    status.textContent = message;
  }
  container.qtiBindDrag({
    sources: '.qti-match-choice', targets: '.qti-match-slot', reorder: false,
    // Drag's third argument is a row-placement direction, not a MATCH move request.
    drop(choice, slot) { assign(choice, slot); }
  });
  container.addEventListener('click', event => {
    const choice = event.target.closest('.qti-match-choice');
    if (choice) {
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
    if (event.key === 'Escape') { select(null); status.textContent = 'Selection cleared.'; return; }
    const slot = event.target.closest('.qti-match-slot');
    if (!slot || event.ctrlKey || event.metaKey || event.altKey || event.isComposing) return;
    const choice = choices.find(button => button.dataset.letter === event.key.toUpperCase());
    if (!choice) return;
    event.preventDefault();
    assign(choice, slot, true);
  });
  container.qtiResetGame = () => {
    container.qtiCancelDrag();
    slots.forEach(emptySlot);
    select(null);
    status.textContent = 'Matches reset.';
  };
})();
</script>
