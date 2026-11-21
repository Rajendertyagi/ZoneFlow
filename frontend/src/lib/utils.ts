/** Join conditional class names. Kept dependency-free for the build-time-only frontend. */
export function cn(...classes: Array<string | false | null | undefined>): string {
  return classes.filter(Boolean).join(' ');
}
