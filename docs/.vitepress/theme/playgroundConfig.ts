export type PlaygroundFormatConfig = {
  lineWidth: number;
  indentWidth: number;
  useTabs: boolean;
};

/** Demo default; Jolt itself defaults to 80 columns. */
export const PLAYGROUND_DEFAULT_CONFIG: PlaygroundFormatConfig = {
  lineWidth: 60,
  indentWidth: 2,
  useTabs: false,
};
