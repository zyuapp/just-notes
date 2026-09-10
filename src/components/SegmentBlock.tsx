import type { ReactNode } from "react";
import { formatDuration } from "../lib/format";
import { segmentClasses, type IndexedSegment } from "../lib/transcript";

type SegmentBlockProps = {
  items: IndexedSegment[];
  query: string;
  active: boolean;
};

export function SegmentBlock({ items, query, active }: SegmentBlockProps) {
  const segment = items[0].segment;
  return (
    <article className={segmentClasses(segment, { isStart: true, isEnd: true }, active)}>
      <header className="segment-head">
        <strong>{segment.speaker}</strong>
        <time>{formatDuration(segment.startMs)}</time>
      </header>
      <p>{items.map((item, index) => (
        <span key={`${item.segment.source}-${item.segment.startMs}-${item.index}`}
          className={active && index === items.length - 1 ? "segment-text active" : "segment-text"}>
          {index > 0 && " "}{highlight(item.segment.text, query)}
        </span>
      ))}</p>
    </article>
  );
}

function highlight(text: string, query: string): ReactNode {
  const needle = query.trim().toLowerCase();
  if (!needle) return text;
  const parts: ReactNode[] = [];
  const haystack = text.toLowerCase();
  let start = 0;
  let match = haystack.indexOf(needle);
  while (match !== -1) {
    parts.push(text.slice(start, match));
    parts.push(<mark key={match}>{text.slice(match, match + needle.length)}</mark>);
    start = match + needle.length;
    match = haystack.indexOf(needle, start);
  }
  parts.push(text.slice(start));
  return parts;
}
