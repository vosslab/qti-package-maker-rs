/*
 * Shared self-test controls. checkAnswer_<CRC> is a host contract: sites may
 * wrap it to record question-level completion. Keep that function stable and
 * resolve its box when called so a re-rendered question cannot grade stale DOM.
 */
(() => {
  const fibNorm = value => String(value ?? '').trim().toLowerCase();
  const multiNorm = value => fibNorm(value)
    .replace(/,/g, '')
    .replace(/\s+/g, '')
    .replace(/(?:cm|mapunits)$/i, '');
  const decode = value => {
    try { return atob(value); } catch { return ''; }
  };

  const boxFor = crc => document.getElementById(`question_html_${crc}`);
  const feedback = (box, text, correct) => {
    const result = box.querySelector('.qti-feedback-result');
    result.className = `qti-feedback-result${
      correct === null ? '' : correct ? ' qti-feedback-success' : ' qti-feedback-error'
    }`;
    result.textContent = text;
    box.querySelector('[role=status]').textContent = text;
  };
  const clearFeedback = box => {
    box.querySelectorAll('.feedback').forEach(cell => {
      cell.textContent = '';
      cell.style.backgroundColor = 'transparent';
      cell.removeAttribute('aria-label');
    });
    feedback(box, '', null);
  };
  const grade = box => {
    const { kind, crc } = box.dataset;
    if (kind === 'mc') {
      const selected = box.querySelector('input[type=radio]:checked');
      const correct = selected?.dataset.correct === 'true';
      feedback(box, !selected ? 'Please select an answer.' : correct ? 'CORRECT' : 'incorrect',
        !selected ? null : correct);
      return;
    }
    if (kind === 'ma') {
      const options = [...box.querySelectorAll('input[type=checkbox]')];
      const selected = options.filter(option => option.checked);
      const right = options.filter(option => option.dataset.correct === 'true');
      const count = selected.filter(option => option.dataset.correct === 'true').length;
      if (!selected.length) {
        feedback(box, 'Please select an answer.', null);
      } else if (count === right.length && selected.length === right.length) {
        feedback(box, 'CORRECT', true);
      } else if (selected.length > right.length) {
        feedback(box, `Too many answers selected. You selected ${count} correct answers, but also included ${selected.length - count} incorrect choices.`, false);
      } else if (count < right.length && selected.length < right.length) {
        feedback(box, `Too few answers selected. You got ${count} out of ${right.length} correct.`, false);
      } else {
        feedback(box, `You selected the right number of choices, but only ${count} out of ${right.length} are correct.`, false);
      }
      return;
    }
    if (kind === 'fib') {
      const input = box.querySelector('.qti-fib-input');
      const correct = decode(input.dataset.answers).split('\u001f')
        .map(fibNorm).includes(fibNorm(input.value));
      feedback(box, correct ? 'CORRECT' : 'incorrect', correct);
      return;
    }
    if (kind === 'num') {
      const input = box.querySelector('.qti-num-input');
      const text = input.value.trim();
      if (!text) { feedback(box, 'Please enter a value.', null); return; }
      const value = Number(text);
      if (!Number.isFinite(value)) { feedback(box, 'Please enter a valid number.', null); return; }
      const answer = Number(box.dataset.answer);
      const tolerance = Number(box.dataset.tolerance);
      const correct = value >= answer - tolerance && value <= answer + tolerance;
      feedback(box, correct ? 'CORRECT' : value > answer + tolerance
        ? 'Too high. Try again.' : value < answer - tolerance
          ? 'Too low. Try again.' : 'Incorrect. Try again.', correct);
      return;
    }
    if (kind === 'multi-fib') {
      const inputs = [...box.querySelectorAll('.fib-blank')];
      let count = 0;
      inputs.forEach(input => {
        const value = multiNorm(input.value);
        const correct = value !== '' && decode(input.dataset.answers).split('\u001f')
          .map(multiNorm).includes(value);
        input.classList.toggle('correct', correct);
        input.classList.toggle('incorrect', !correct);
        count += Number(correct);
      });
      const correct = count === inputs.length;
      feedback(box, correct ? 'CORRECT' : `Correct: ${count} of ${inputs.length}`, correct);
      return;
    }
    const rows = [...box.querySelectorAll(kind === 'match' ? '.qti-match-slot' : '.qti-order-row')];
    let score = 0;
    rows.forEach((row, index) => {
      const correct = row.dataset.value === (kind === 'match'
        ? row.dataset.correct : `${crc}_${String(index + 1).padStart(3, '0')}`);
      const cell = kind === 'match'
        ? row.closest('tr').querySelector('.feedback') : row.querySelector('.feedback');
      score += Number(correct);
      cell.textContent = correct ? '\u2705' : '\u274c';
      cell.setAttribute('aria-label', correct ? 'Correct' : 'Incorrect');
      cell.style.backgroundColor = correct ? 'var(--qti-success-bg)' : 'var(--qti-error-bg)';
    });
    const correct = score === rows.length;
    feedback(box, kind === 'match' ? `Total Score: ${score} out of ${rows.length}`
      : `Correct positions: ${score} of ${rows.length}`, correct);
  };

  const bindButtonFeedback = () => {
    if (window.__qtiSelftestButtonFeedbackBound) return;
    window.__qtiSelftestButtonFeedbackBound = true;
    let pressed;
    const buttonFor = event => event.target instanceof Element
      ? event.target.closest('.qti-selftest .qti-btn') : null;
    const release = () => { pressed?.classList.remove('qti-pressed'); pressed = undefined; };
    document.addEventListener('pointerdown', event => {
      if (event.button !== 0) return;
      release();
      const button = buttonFor(event);
      if (!button || button.disabled) return;
      pressed = button;
      pressed.classList.add('qti-pressed');
    });
    document.addEventListener('pointerup', release);
    document.addEventListener('pointercancel', release);
    document.addEventListener('keydown', event => {
      if (![' ', 'Enter'].includes(event.key) || event.isComposing || event.ctrlKey || event.metaKey || event.altKey) return;
      release();
      const button = buttonFor(event);
      if (!button || button.disabled) return;
      pressed = button;
      pressed.classList.add('qti-pressed');
    });
    document.addEventListener('keyup', event => { if ([' ', 'Enter'].includes(event.key)) release(); });
    window.addEventListener('blur', release);
  };

  const initialize = box => {
    if (box.qtiControlsInitialized) return;
    box.qtiControlsInitialized = true;
    const { crc, kind } = box.dataset;
    const checkName = `checkAnswer_${crc}`;
    const clearName = `clearFeedback_${crc}`;
    // Do not replace a website wrapper. The default functions look up the
    // current box at invocation time, which also keeps existing wrappers live.
    if (typeof window[checkName] !== 'function') {
      window[checkName] = () => {
        const current = boxFor(crc);
        if (current) grade(current);
      };
    }
    if (typeof window[clearName] !== 'function') {
      window[clearName] = () => {
        const current = boxFor(crc);
        if (current) clearFeedback(current);
      };
    }
    box.addEventListener('click', event => {
      if (event.target.closest('[data-action=grade]')) {
        window[checkName]?.();
      } else if (event.target.closest('[data-action=reset]')) {
        // Clear the score first so game reset announcements remain available to screen readers.
        clearFeedback(box);
        if (box.qtiResetGame) box.qtiResetGame();
        else if (kind === 'ma') box.querySelectorAll('input[type=checkbox]')
          .forEach(input => { input.checked = false; });
      }
    });
    box.addEventListener('keydown', event => {
      if (event.key === 'Enter' && event.target.matches('.qti-num-input')) {
        event.preventDefault();
        window[checkName]?.();
      } else if (event.key === 'Enter' && event.target.matches('.fib-blank')) {
        event.preventDefault();
      }
    });
  };

  bindButtonFeedback();
  document.querySelectorAll('.qti-selftest-item').forEach(initialize);
})();
