export function ErrorBanner({ message }: { message: string }) {
  return (
    <div className="error-banner" role="alert">
      <pre>{message}</pre>
    </div>
  );
}
