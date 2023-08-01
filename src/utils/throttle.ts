export const throttle = <T extends unknown[]>(
  callback: (...args: T) => void,
  wait: number,
) => {
  let timeoutActive = false;
  let pending: T | null = null;
  return (...args: T) => {
    if (timeoutActive) {
      pending = args;
      return;
    }
    timeoutActive = true;
    callback(...args);
    const interval = setInterval(() => {
      if (pending) {
        const args = pending;
        pending = null;
        callback(...args);
      } else {
        clearInterval(interval);
        timeoutActive = false;
      }
    }, wait);
  };
};
