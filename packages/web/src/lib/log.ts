const COLORS = {
  reset: '\x1b[0m',
  info: '\x1b[36m',
  warn: '\x1b[33m',
  error: '\x1b[31m',
  debug: '\x1b[35m',
  gray: '\x1b[90m',
} as const;

type LogLevel = keyof typeof COLORS | 'reset' | 'gray';

const getTimestamp = (): string => {
  const now = new Date();
  return now.toISOString();
};

const formatMessage = (
  level: keyof typeof COLORS,
  ...args: unknown[]
): string => {
  const timestamp = getTimestamp();
  const color = COLORS[level];
  const reset = COLORS.reset;
  const gray = COLORS.gray;
  const prefix = `${gray}[${timestamp}]${reset} ${color}[${level.toUpperCase()}]${reset}`;
  const message = args
    .map((arg) =>
      typeof arg === 'object' && arg !== null
        ? JSON.stringify(arg)
        : String(arg)
    )
    .join(' ');
  return `${prefix} ${message}`;
};

export const log = {
  info: (...args: unknown[]): void => {
    console.log(formatMessage('info', ...args));
  },
  warn: (...args: unknown[]): void => {
    console.warn(formatMessage('warn', ...args));
  },
  error: (...args: unknown[]): void => {
    console.error(formatMessage('error', ...args));
  },
  debug: (...args: unknown[]): void => {
    if (process.env.NODE_ENV !== 'production') {
      console.debug(formatMessage('debug', ...args));
    }
  },
};

export default log;
