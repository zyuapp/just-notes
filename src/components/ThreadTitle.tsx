import { useEffect, useState } from "react";

type ThreadTitleProps = {
  title: string | null;
  canRename: boolean;
  onRename: (title: string) => void;
};

export function ThreadTitle({ title, canRename, onRename }: ThreadTitleProps) {
  const [draft, setDraft] = useState<string | null>(null);

  useEffect(() => {
    setDraft(null);
  }, [title]);

  if (title === null) {
    return <h1>No thread selected</h1>;
  }

  if (draft === null) {
    return (
      <h1>
        <button
          type="button"
          className="thread-title-button"
          onClick={() => canRename && setDraft(title)}
          title={canRename ? "Rename thread" : undefined}
        >
          {title}
        </button>
      </h1>
    );
  }

  const commit = () => {
    const next = draft.trim();
    setDraft(null);
    if (next && next !== title) {
      onRename(next);
    }
  };

  return (
    <input
      className="thread-title-input"
      value={draft}
      autoFocus
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === "Enter") commit();
        if (event.key === "Escape") setDraft(null);
      }}
      aria-label="Thread title"
    />
  );
}
