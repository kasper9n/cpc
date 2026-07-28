<script lang="ts">
	import { check_shortcut } from "$lib/helpers";
	import { flip } from "svelte/animate";
	import { fly } from "svelte/transition";
	import { PersistedState } from "runed";
    import { tick } from "svelte";

	// Has to be dynamically imported for prerendering to work
	// https://github.com/sveltejs/svelte/issues/13155
	const cpc_promise = import("cpc");
	let cpc: typeof import("cpc") | undefined;

	// Initialize currency cache when cpc module is loaded
	cpc_promise.then(async (mod) => {
		mod.set_settings({
			locale: Intl?.DateTimeFormat()?.resolvedOptions()?.locale ?? navigator.language
		})
		cpc = mod;

		// Fetch exchange rates and initialize currency cache
		try {
			const response = await fetch("https://api.frankfurter.dev/v2/rates?base=EUR");
			const ratesJson = await response.text();
			if (typeof mod.init_currency_cache_with_json !== 'function') {
				output = 'Error: Missing currency init function'
			}
				mod.init_currency_cache_with_json(ratesJson);
		} catch (e) {
			console.error("Failed to fetch currency rates:", e);
			output = "Error: Failed to fetch currency rates:", e
		}
	});

	function wasm_eval() {
		if (input === '' || !cpc || input.trim().length === 0) {
			return "";
		}
		try {
			return cpc.wasm_eval(input);
		} catch (e) {
			return "";
		}
	}

	let input = $state("");
	let output = $derived(wasm_eval());
	let calc_history = new PersistedState<
		{ id: number; in: string; out: string }[]
	>("calc_history", []);
</script>

<svelte:head>
	<title>cpc</title>
	<meta
		name="description"
		content="Text calculator with support for units and conversion"
	/>
</svelte:head>

<main class="w-full px-4 lg:px-8 text-base lg:text-lg">
	<nav class="flex items-center justify-between py-4 lg:py-6">
		<h1 class="text-3xl font-bold text-amber-600 dark:text-amber-400">cpc</h1>
		<div class="flex items-center gap-4">
			{#if calc_history.current.length > 0}
				<button
					type="button"
					class="text-sm p-2 opacity-65 hover:opacity-100 transition-opacity ease-out duration-150 cursor-pointer"
					onclick={() => {
						calc_history.current = [];
					}}
				>
					Clear history
				</button>
			{/if}
			<a
				href="https://github.com/kasper9n/cpc"
				aria-label="GitHub repository"
				class="svelte-1ugh5mt"
			>
				<svg
					height="24"
					viewBox="-2 -2 28 28"
					width="24"
					xmlns="http://www.w3.org/2000/svg"
					class="svelte-8lfi33 svelte-1ugh5mt"
					><path
						d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"
						fill="none"
						stroke="currentColor"
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						class="svelte-8lfi33"
					></path></svg
				>
			</a>
		</div>
	</nav>
	<!-- svelte-ignore a11y_autofocus -->
	<input
		type="text"
		class="border border-gray-500/50 w-full rounded-lg px-3 py-2 outline-none"
		bind:value={input}
		onkeydown={async (e) => {
			const input_el = e.currentTarget;
			if (check_shortcut(e, "Enter")) {
				const out = wasm_eval();
				console.log(out);
				calc_history.current.push({
					id: Date.now(),
					in: input,
					out,
				});
				input_el.value = "";
				output = "";
			}
		}}
		placeholder="10km/h * 1 decade in light seconds"
		autofocus
	/>
	<div class="pt-1 leading-tight">
		<div class="px-3 py-2">
			{output}<span class="invisible select-none">x</span>
		</div>
		{#each calc_history.current as _, i (calc_history.current.length - 1 - i)}
			{let query = calc_history.current[calc_history.current.length - 1 - i]}
			<div
				class="px-3 py-2"
				in:fly={{ y: -10, duration: 150 }}
				animate:flip={{ duration: 150 }}
			>
				<p class="opacity-65 text-base leading-tight">{query.in}</p>
				<p>{query.out}</p>
			</div>
		{/each}
	</div>
</main>

<style lang="postcss">
	@reference "../app.css";

	@media (prefers-color-scheme: dark) {
		:global(body) {
			@apply bg-black text-white;
		}
	}
</style>
