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
  devices: [], // enabled controller ids, in the order they were checked
  mapping: { byDevice: {} }, // per controller: { ports: {port -> {button -> input}}, layout, note } for the current core
  options: { coreId: null, values: {} }, // core-specific option values for the selected core
  optionMemory: {}, // coreId -> { optionId: value } the user has set, restored when they switch back
};