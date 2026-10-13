<script>
(function() {
  const container = document.getElementById('question_html_{{CRC}}');
  if (!container || container.qpmDragInitialized) return;
  container.qpmDragInitialized = true;
  container.qpmBindDrag = ({sources, targets, reorder, drop}) => {
    let active = null;
    function clearTargets() {
      container.querySelectorAll(targets).forEach(target => {
        target.classList.remove('qpm-drop-target', 'qpm-drop-before', 'qpm-drop-after');
      });
    }
    function cancel() {
      if (active) active.classList.remove('qpm-dragging');
      active = null;
      clearTargets();
    }
    function after(event, target) {
      const bounds = target.getBoundingClientRect();
      return event.clientY > bounds.top + bounds.height / 2;
    }
    container.addEventListener('dragstart', event => {
      const source = event.target.closest(sources);
      if (!source || source.disabled || source.getAttribute('draggable') !== 'true') {
        event.preventDefault();
        return;
      }
      active = source;
      event.dataTransfer.setData('text/plain', source.dataset.value);
      event.dataTransfer.effectAllowed = reorder ? 'move' : 'copy';
      source.classList.add('qpm-dragging');
    });
    container.addEventListener('dragover', event => {
      const target = event.target.closest(targets);
      if (!active || !target || target === active) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = reorder ? 'move' : 'copy';
      clearTargets();
      target.classList.add('qpm-drop-target');
      if (reorder) target.classList.add(after(event, target) ? 'qpm-drop-after' : 'qpm-drop-before');
    });
    container.addEventListener('dragleave', event => {
      const target = event.target.closest(targets);
      if (target && !target.contains(event.relatedTarget)) {
        target.classList.remove('qpm-drop-target', 'qpm-drop-before', 'qpm-drop-after');
      }
    });
    container.addEventListener('drop', event => {
      const target = event.target.closest(targets);
      if (!target) return;
      event.preventDefault();
      const source = active;
      const valid = source && !source.disabled && target !== source &&
        event.dataTransfer.getData('text/plain') === source.dataset.value;
      cancel();
      if (valid) drop(source, target, after(event, target));
    });
    container.addEventListener('dragend', cancel);
    container.qpmCancelDrag = cancel;
  };
})();
</script>
