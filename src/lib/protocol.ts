export function customProtocolUrl(scheme: string, path: string): string {
  const normalizedPath = path.replace(/^\/+/, "");
  if (navigator.userAgent.includes("Windows")) {
    return `http://${scheme}.localhost/${normalizedPath}`;
  }
  return `${scheme}://localhost/${normalizedPath}`;
}
