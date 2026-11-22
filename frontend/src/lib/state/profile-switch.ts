/**
 * U-07: the compact profile selector shown below `lg` (where the rail is
 * hidden) must call the exact same load action as the rail's ProfilesPanel.
 * The options are centralised here so both surfaces cannot drift, and kept
 * free of Svelte/API imports so the mapping is unit-testable with `bun test`.
 */
export interface ProfileLoadOptions {
  profile: string;
  action: string;
  successMessage: string;
}

export function compactProfileLoadOptions(name: string): ProfileLoadOptions {
  return {
    profile: name,
    action: `switch to ${name}`,
    successMessage: `Loaded ${name} into the editor draft.`,
  };
}
