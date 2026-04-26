type ErrorToastProps = {
  message: string | null;
};

export function ErrorToast({ message }: ErrorToastProps) {
  return message ? <pre className="error">{message}</pre> : null;
}
