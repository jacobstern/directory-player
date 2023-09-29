export function debounce<T extends unknown[]>(
  callback: (...args: T) => void,
  wait: number,
): (...args: T) => void {
  let timeout: number | undefined;
  return (...args: T) => {
    if (timeout !== undefined) {
      window.clearTimeout(timeout);
    }
    timeout = setTimeout(() => {
      callback(...args);
    }, wait);
  };
}

export default debounce;
