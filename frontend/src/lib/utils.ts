import { twMerge } from 'tailwind-merge';

/**
 * Join conditional class names, letting later (caller) classes win over earlier
 * (component default) ones. `twMerge` resolves Tailwind conflicts by class
 * group, so a caller passing `h-8 w-52` to an input defaulting to `h-9 w-full`
 * gets exactly the caller's sizing instead of whichever rule CSS source order
 * happens to favour (U-02).
 */
export function cn(...classes: Array<string | false | null | undefined>): string {
  return twMerge(classes.filter(Boolean).join(' '));
}
