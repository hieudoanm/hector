'use client';

import type { FC } from 'react';

const GlobalError: FC<{
  error: Error & { digest?: string };
  reset: () => void;
}> = ({ error, reset }) => {
  return (
    <html lang="en">
      <body>
        <div className="flex min-h-screen flex-col items-center justify-center gap-4 p-8">
          <h1 className="text-2xl font-bold">Something went wrong</h1>
          <p className="max-w-md text-center text-sm text-gray-600 dark:text-gray-400">
            {error.message}
          </p>
          <button
            onClick={reset}
            className="cursor-pointer rounded bg-blue-600 px-4 py-2 text-white">
            Try again
          </button>
        </div>
      </body>
    </html>
  );
};

export default GlobalError;
