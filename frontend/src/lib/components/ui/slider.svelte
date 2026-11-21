<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    value?: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    id?: string;
    class?: string;
    onchange?: (value: number) => void;
  }

  let {
    value = $bindable(0),
    min = 0,
    max = 100,
    step = 1,
    disabled = false,
    id,
    class: className = '',
    onchange,
  }: Props = $props();

  function handleInput(event: Event): void {
    value = Number((event.currentTarget as HTMLInputElement).value);
    onchange?.(value);
  }
</script>

<input
  {id}
  type="range"
  {min}
  {max}
  {step}
  {disabled}
  {value}
  oninput={handleInput}
  class={cn(
    'h-1.5 w-full cursor-pointer appearance-none rounded-full bg-zf-border-strong accent-zf-accent',
    'disabled:cursor-not-allowed disabled:opacity-50',
    className,
  )}
/>
