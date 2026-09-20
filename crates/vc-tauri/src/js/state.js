// Shared mutable state for the build wizard. A single plain object is
// enough here -- every module that touches it runs in the same page, so
// there's no need for a store/pubsub layer on top.

export const state = {
  step: 0,
  cores: [],
  rom: null, // { path, name }
  cover: null, // { path, name }
  donor: null, // { path, name }
  keys: null, // { path, name }
  mapping: { device: null, ports: {}, layout: "", note: "" }, // bindings per emulated controller (port -> button -> input) for the current core + controller
  options: { coreId: null, values: {} }, // core-specific option values for the selected core
  optionMemory: {}, // coreId -> { optionId: value } the user has set, restored when they switch back
};