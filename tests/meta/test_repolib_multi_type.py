"""Behavioral tests for multi-type REPO_TYPE markers (comma-separated, and 'all')."""

import os
import pathlib

import file_utils
import repolib.files
import repolib.gitignore
import repolib.plan
import repolib.model


class TestEffectiveTypeChainUnion:
	"""effective_type_chain() over a comma marker unions and orders single-token chains."""

	def test_union_covers_each_declared_family(self) -> None:
		"""The multi-type chain is a superset of each single-token chain it declares."""
		combined = repolib.model.effective_type_chain('python,rust')
		python_only = repolib.model.effective_type_chain('python')
		rust_only = repolib.model.effective_type_chain('rust')

		assert set(python_only) <= set(combined)
		assert set(rust_only) <= set(combined)

	def test_declaration_order_preserved(self) -> None:
		"""'python' precedes 'rust' when declared in that order."""
		combined = repolib.model.effective_type_chain('python,rust')

		assert combined.index('python') < combined.index('rust')

	def test_dedupe_idempotent(self) -> None:
		"""Repeating the same token in a marker changes nothing."""
		assert repolib.model.effective_type_chain('python,python') == repolib.model.effective_type_chain('python')

	def test_all_covers_every_known_type(self) -> None:
		"""'all' expands to every declared token in KNOWN_REPO_TYPES except 'all' itself."""
		combined = set(repolib.model.effective_type_chain('all'))
		expected_tokens = repolib.model.KNOWN_REPO_TYPES - {repolib.model.LANG_ALL}

		assert expected_tokens <= combined


class TestValidateMarker:
	"""validate_marker() drops unknown tokens but degrades only when nothing is known."""

	def test_typo_preserves_the_valid_half(self) -> None:
		"""A marker with one good and one bad token routes on the good token alone."""
		canonical = repolib.model.validate_marker('python,pyhton', 'test-repo')
		chain = repolib.model.effective_type_chain(canonical)

		assert 'python' in chain
		assert repolib.model.LANG_OTHER not in chain

	def test_wholly_invalid_marker_degrades_to_other(self) -> None:
		"""A marker with no recognized token routes as 'other'."""
		canonical = repolib.model.validate_marker('pyhton', 'test-repo')

		assert canonical == repolib.model.LANG_OTHER


class TestFindSourceForBucketPrecedence:
	"""First-declared-type-wins precedence, proven at actual source resolution."""

	def _build_tree(self, tmp_path: pathlib.Path) -> str:
		# Same relative doc path exists under both a python and a rust overlay.
		root = tmp_path / "template"
		python_docs = root / "templates" / "python" / "docs"
		rust_docs = root / "templates" / "rust" / "docs"
		python_docs.mkdir(parents=True)
		rust_docs.mkdir(parents=True)
		(python_docs / "SHARED.md").write_text('python version\n', encoding='utf-8')
		(rust_docs / "SHARED.md").write_text('rust version\n', encoding='utf-8')
		return str(root)

	def test_python_first_wins(self, tmp_path: pathlib.Path) -> None:
		"""'python,rust' resolves the collision to the python overlay copy."""
		root = self._build_tree(tmp_path)

		source = repolib.model.find_source_for_bucket(root, 'overwrite_files', 'docs/SHARED.md', 'python,rust')

		assert source == os.path.join(root, 'templates', 'python', 'docs', 'SHARED.md')

	def test_rust_first_wins(self, tmp_path: pathlib.Path) -> None:
		"""'rust,python' resolves the same collision to the rust overlay copy."""
		root = self._build_tree(tmp_path)

		source = repolib.model.find_source_for_bucket(root, 'overwrite_files', 'docs/SHARED.md', 'rust,python')

		assert source == os.path.join(root, 'templates', 'rust', 'docs', 'SHARED.md')


class TestGitignoreBlockUnion:
	"""compute_propagation_plan()'s gitignore_block unions every declared type's lines."""

	def test_typed_blocks_union_across_declared_types(self) -> None:
		"""A 'python,rust' plan's gitignore_block is a superset of each single-type block."""
		template_root = file_utils.get_repo_root()

		combined_plan = repolib.plan.compute_propagation_plan(template_root, 'python,rust')
		python_plan = repolib.plan.compute_propagation_plan(template_root, 'python')
		rust_plan = repolib.plan.compute_propagation_plan(template_root, 'rust')

		assert set(python_plan['gitignore_block']) <= set(combined_plan['gitignore_block'])
		assert set(rust_plan['gitignore_block']) <= set(combined_plan['gitignore_block'])

	def test_all_keeps_typed_blocks(self) -> None:
		"""'all' still carries every typed block into gitignore_block.

		This property already held before the multi-type rewrite (the old 'all'
		recursion loaded each child's typed block too); it guards against the
		rewrite losing the union, not against a gap being closed here.
		"""
		template_root = file_utils.get_repo_root()

		all_plan = repolib.plan.compute_propagation_plan(template_root, 'all')
		python_plan = repolib.plan.compute_propagation_plan(template_root, 'python')

		assert set(python_plan['gitignore_block']) <= set(all_plan['gitignore_block'])


class TestMergeGitignoreBlocksMultiType:
	"""merge_gitignore_blocks() writes one managed block per declared type, stably."""

	def _run_gitignore_pipeline(
			self, repo_dir: pathlib.Path, context: repolib.model.PropagateContext,
			counters: dict,
			) -> None:
		"""Run the same gitignore normalization sequence as process_repo()."""
		template_root = file_utils.get_repo_root()
		repolib.files.merge_gitignore_blocks(
			str(repo_dir), 'python', template_root, context, counters=counters,
		)
		replacements = repolib.files.load_gitignore_replacements(
			'meta/propagation/gitignore_replacements.txt', template_root,
		)
		repolib.files.replace_gitignore_entries(
			str(repo_dir / '.gitignore'), replacements, dry_run=False,
		)
		repolib.files.deduplicate_gitignore(
			str(repo_dir / '.gitignore'), dry_run=False, counters=counters,
		)

	def test_writes_both_blocks_and_is_idempotent(self, tmp_path: pathlib.Path) -> None:
		"""A 'python,rust' merge yields both headers, and a second run changes nothing."""
		template_root = file_utils.get_repo_root()
		repo_dir = tmp_path / "repo"
		repo_dir.mkdir()
		context = repolib.model.PropagateContext(
			source_dir=str(repo_dir),
			template_root=template_root,
			repo_name=None,
			dry_run=False,
			initial_setup=False,
			auto_discover=False,
			write_marker=False,
		)

		repolib.files.merge_gitignore_blocks(str(repo_dir), 'python,rust', template_root, context)
		first_content = (repo_dir / ".gitignore").read_bytes()
		repolib.files.merge_gitignore_blocks(str(repo_dir), 'python,rust', template_root, context)
		second_content = (repo_dir / ".gitignore").read_bytes()

		assert b'# === PYTHON ===' in first_content and b'# === RUST ===' in first_content
		assert first_content == second_content

	def test_single_type_output_is_stable(self, tmp_path: pathlib.Path) -> None:
		"""A single-type marker still writes UNIVERSAL plus its one block, idempotently."""
		template_root = file_utils.get_repo_root()
		repo_dir = tmp_path / "repo"
		repo_dir.mkdir()
		context = repolib.model.PropagateContext(
			source_dir=str(repo_dir),
			template_root=template_root,
			repo_name=None,
			dry_run=False,
			initial_setup=False,
			auto_discover=False,
			write_marker=False,
		)

		repolib.files.merge_gitignore_blocks(str(repo_dir), 'python', template_root, context)
		first_content = (repo_dir / ".gitignore").read_bytes()
		repolib.files.merge_gitignore_blocks(str(repo_dir), 'python', template_root, context)
		second_content = (repo_dir / ".gitignore").read_bytes()

		assert b'# === PYTHON ===' in first_content and b'# === RUST ===' not in first_content
		assert first_content == second_content

	def test_local_alias_does_not_report_repeat_churn(self, tmp_path: pathlib.Path) -> None:
		"""A local alias converges into the managed block and stays unchanged."""
		template_root = file_utils.get_repo_root()
		repo_dir = tmp_path / "repo"
		repo_dir.mkdir()
		(repo_dir / '.gitignore').write_text('node_modules/\n', encoding='utf-8')
		context = repolib.model.PropagateContext(
			source_dir=str(repo_dir),
			template_root=template_root,
			repo_name=None,
			dry_run=False,
			initial_setup=False,
			auto_discover=False,
			write_marker=False,
		)

		first_counters = {'created_count': 0, 'merged_count': 0}
		self._run_gitignore_pipeline(repo_dir, context, first_counters)
		first_content = (repo_dir / '.gitignore').read_text(encoding='utf-8')
		managed_header = repolib.gitignore.managed_gitignore_header('UNIVERSAL')
		local_content, managed_content = first_content.split(managed_header, maxsplit=1)
		second_counters = {'merged_count': 0}
		self._run_gitignore_pipeline(repo_dir, context, second_counters)

		assert '/node_modules/' not in local_content and '/node_modules/' in managed_content
		assert second_counters['merged_count'] == 0

	def _context(self, repo_dir: pathlib.Path) -> repolib.model.PropagateContext:
		"""Build a writable propagation context for a disposable repository."""
		return repolib.model.PropagateContext(
			source_dir=str(repo_dir),
			template_root=file_utils.get_repo_root(),
			repo_name=None,
			dry_run=False,
			initial_setup=False,
			auto_discover=False,
			write_marker=False,
		)

	def _propagated_headers(self, repo_type: str) -> list[str]:
		"""Return this marker's rendered managed headings from live template data."""
		template_root = file_utils.get_repo_root()
		headers = [repolib.gitignore.managed_gitignore_header('UNIVERSAL')]
		for declared_type in repolib.model.expand_marker_types(repo_type):
			block_path = pathlib.Path(template_root, 'templates', declared_type, f'gitignore.{declared_type}')
			if repolib.files.load_gitignore_block(str(block_path)):
				headers.append(repolib.gitignore.managed_gitignore_header(declared_type.upper()))
		return headers

	def _render_gitignore(self, repo_dir: pathlib.Path, repo_type: str) -> bytes:
		"""Render a disposable .gitignore through its production entry point."""
		repolib.files.merge_gitignore_blocks(
			str(repo_dir), repo_type, file_utils.get_repo_root(), self._context(repo_dir),
		)
		return (repo_dir / '.gitignore').read_bytes()

	def _local_body(self, rendered_lines: list[str]) -> list[str]:
		"""Extract consumer-owned lines after the renderer-owned LOCAL banner."""
		local_header_index = rendered_lines.index(repolib.gitignore.GITIGNORE_LOCAL_HEADER)
		assert rendered_lines[local_header_index - 1] == repolib.gitignore.GITIGNORE_LOCAL_RULE
		assert rendered_lines[local_header_index + 1] == repolib.gitignore.GITIGNORE_LOCAL_NOTICE
		return rendered_lines[local_header_index + 2:]

	def test_local_section_converges_last_without_losing_its_body(self, tmp_path: pathlib.Path) -> None:
		"""Every historical LOCAL placement preserves its body and ends after live blocks."""
		repo_type = 'python,rust'
		body = ['', '# consumer-owned comment', 'consumer/cache/', '', '!consumer/keep.txt']
		propagated_headers = self._propagated_headers(repo_type)
		assert len(propagated_headers) > 1
		old_universal = ['# === UNIVERSAL ===', 'obsolete-universal/']
		first_declared_type = repolib.model.expand_marker_types(repo_type)[0]
		old_typed = [f'# === {first_declared_type.upper()} ===', 'obsolete-typed/']
		current_local = [
			repolib.gitignore.GITIGNORE_LOCAL_HEADER,
			repolib.gitignore.GITIGNORE_LOCAL_NOTICE,
			*body,
		]
		previous_local = [
			repolib.gitignore.GITIGNORE_PREVIOUS_LOCAL_HEADER,
			repolib.gitignore.GITIGNORE_LOCAL_NOTICE,
			*body,
		]
		legacy_local = [repolib.gitignore.GITIGNORE_LEGACY_LOCAL_HEADER, *body]
		divided_local = [
			repolib.gitignore.GITIGNORE_LOCAL_RULE,
			repolib.gitignore.GITIGNORE_LOCAL_HEADER,
			repolib.gitignore.GITIGNORE_LOCAL_NOTICE,
			repolib.gitignore.GITIGNORE_LOCAL_RULE_END,
			*body,
		]
		inputs = {
			'legacy': [*legacy_local, *old_universal, *old_typed],
			'divided': [*old_universal, *divided_local, *old_typed],
			'top': [*current_local, *old_universal, *old_typed],
			'middle': [*old_universal, *current_local, *old_typed],
			'previous-top': [*previous_local, *old_universal, *old_typed],
			'previous-middle': [*old_universal, *previous_local, *old_typed],
			'absent': [*body, *old_universal, *old_typed],
		}
		renders = []

		for placement, initial_lines in inputs.items():
			repo_dir = tmp_path / placement
			repo_dir.mkdir()
			(repo_dir / '.gitignore').write_text('\n'.join(initial_lines) + '\n', encoding='utf-8')
			first_render = self._render_gitignore(repo_dir, repo_type)
			second_render = self._render_gitignore(repo_dir, repo_type)
			rendered_lines = first_render.decode('utf-8').splitlines()
			local_header_index = rendered_lines.index(repolib.gitignore.GITIGNORE_LOCAL_HEADER)
			actual_propagated_headers = [
				line for line in rendered_lines
				if '[PROPAGATED - LOCAL EDITS OVERWRITTEN]' in line
			]

			assert actual_propagated_headers == propagated_headers
			assert all(rendered_lines.index(header) < local_header_index for header in propagated_headers)
			assert self._local_body(rendered_lines) == body
			assert repolib.gitignore.GITIGNORE_LOCAL_RULE_END not in rendered_lines
			assert first_render == second_render
			renders.append(first_render)

		assert renders == [renders[0]] * len(renders)

	def test_rebuilds_reverse_managed_order_before_local_section(self, tmp_path: pathlib.Path) -> None:
		"""Reverse legacy blocks converge to live marker order without stale content."""
		repo_type = 'python,rust'
		repo_dir = tmp_path / 'reverse-managed-order'
		repo_dir.mkdir()
		declared_types = repolib.model.expand_marker_types(repo_type)
		reverse_headers = [
			f'# === {declared_type.upper()} ==='
			for declared_type in reversed(declared_types)
		]
		old_blocks = []
		for header in reverse_headers:
			old_blocks.extend([header, f'obsolete-{header.lower()}/'])
		old_blocks.extend(['# === UNIVERSAL ===', 'obsolete-universal/'])
		body = ['# consumer-owned comment', 'consumer/cache/']
		(repo_dir / '.gitignore').write_text(
			'\n'.join([
				*old_blocks,
				repolib.gitignore.GITIGNORE_LEGACY_LOCAL_HEADER,
				*body,
			]) + '\n', encoding='utf-8',
		)

		first_render = self._render_gitignore(repo_dir, repo_type)
		second_render = self._render_gitignore(repo_dir, repo_type)
		rendered_lines = first_render.decode('utf-8').splitlines()
		actual_headers = [
			line for line in rendered_lines
			if '[PROPAGATED - LOCAL EDITS OVERWRITTEN]' in line
		]

		assert actual_headers == self._propagated_headers(repo_type)
		assert self._local_body(rendered_lines) == body
		assert 'obsolete-universal/' not in rendered_lines
		assert all('obsolete-# ===' not in line for line in rendered_lines)
		assert first_render == second_render

	def test_preserves_divider_looking_comments_outside_and_inside_legacy_local_body(
			self, tmp_path: pathlib.Path,
			) -> None:
		"""Old divider spellings remain consumer content without a complete old banner."""
		repo_dir = tmp_path / 'legacy-divider-comments'
		repo_dir.mkdir()
		body = [
			'# -----------------------------------------------------------------------------',
			'# === LOCAL REPOSITORY RULES ===',
			'consumer/before-dividers/',
			'# -----------------------------------------------------------------------------',
			'# =============================================================================',
			'consumer/cache/',
		]
		# Use the model-derived UNIVERSAL marker as the real section boundary.
		(repo_dir / '.gitignore').write_text(
			'\n'.join([*body, '# === UNIVERSAL ===', 'obsolete-universal/']) + '\n',
			encoding='utf-8',
		)

		first_render = self._render_gitignore(repo_dir, 'python')
		second_render = self._render_gitignore(repo_dir, 'python')
		rendered_lines = first_render.decode('utf-8').splitlines()

		assert self._local_body(rendered_lines) == body
		assert first_render == second_render

	def test_preserves_local_heading_prefix_comments(self, tmp_path: pathlib.Path) -> None:
		"""Heading-prefix comments stay in LOCAL unless they form a full banner."""
		repo_dir = tmp_path / 'local-heading-prefix-comments'
		repo_dir.mkdir()
		body = [
			'# ADD YOUR CUSTOM IGNORES BELOW, except this is a comment',
			'consumer/first/',
			'# === LOCAL REPOSITORY RULES ===, except this is a comment',
			'consumer/second/',
		]
		(repo_dir / '.gitignore').write_text(
			'\n'.join([
				repolib.gitignore.GITIGNORE_LEGACY_LOCAL_HEADER,
				*body,
				'# === UNIVERSAL ===',
				'obsolete-universal/',
			]) + '\n',
			encoding='utf-8',
		)

		first_render = self._render_gitignore(repo_dir, 'python')
		second_render = self._render_gitignore(repo_dir, 'python')
		rendered_lines = first_render.decode('utf-8').splitlines()

		assert self._local_body(rendered_lines) == body
		assert first_render == second_render

	def test_consumer_section_comment_does_not_end_local_body(self, tmp_path: pathlib.Path) -> None:
		"""An arbitrary section-looking comment preserves the LOCAL body's order."""
		repo_dir = tmp_path / 'consumer-divider'
		repo_dir.mkdir()
		body = ['rule_a/', '# === consumer divider', 'rule_b/']
		(repo_dir / '.gitignore').write_text(
			'\n'.join([
				repolib.gitignore.GITIGNORE_LEGACY_LOCAL_HEADER,
				*body,
				'# === UNIVERSAL ===',
				'obsolete-universal/',
			]) + '\n',
			encoding='utf-8',
		)

		first_render = self._render_gitignore(repo_dir, 'python')
		second_render = self._render_gitignore(repo_dir, 'python')
		rendered_lines = first_render.decode('utf-8').splitlines()

		assert self._local_body(rendered_lines) == body
		assert first_render == second_render


class TestSpacedBlock:
	"""spaced_block() normalizes a managed block's trailing blank line."""

	def test_collapses_existing_trailing_blanks(self) -> None:
		"""An older block with several trailing blanks converges to exactly one."""
		assert repolib.gitignore.spaced_block(['a', 'b', '', '', '']) == ['a', 'b', '']

	def test_adds_one_trailing_blank(self) -> None:
		"""A block with no trailing blank gains exactly one."""
		assert repolib.gitignore.spaced_block(['a', 'b']) == ['a', 'b', '']
