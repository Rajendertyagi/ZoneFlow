<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  interface Props {
    for?: string;
    class?: string;
    children?: Snippet;
  }

  let { for: htmlFor, class: className = '', children }: Props = $props();
</script>

<label
  for={htmlFor}
  class={cn('text-xs font-medium text-muted-foreground', className)}
>
  {@render children?.()}
</label>
