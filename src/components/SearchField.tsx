import { Search } from "lucide-react";

type SearchFieldProps = {
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  className?: string;
};

export function SearchField({ value, placeholder, onChange, className }: SearchFieldProps) {
  return (
    <label className={["ui-search", className].filter(Boolean).join(" ")}>
      <Search size={13} aria-hidden="true" />
      <input
        type="search"
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
        aria-label={placeholder}
      />
    </label>
  );
}
