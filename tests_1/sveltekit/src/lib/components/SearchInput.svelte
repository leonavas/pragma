<script lang="ts">
	type Props = { value?: string; label: string; placeholder: string };
	let { value = $bindable(''), label, placeholder }: Props = $props();

	const id = $props.id();
</script>

<div class="field">
	<label for={id}>{label}</label>
	<input {id} type="search" autocomplete="off" {placeholder} bind:value />
</div>
