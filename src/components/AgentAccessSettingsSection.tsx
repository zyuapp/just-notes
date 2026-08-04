import { FolderOpen, LoaderCircle } from "lucide-react";
import { BsOpenai } from "react-icons/bs";
import { SiClaudecode } from "react-icons/si";
import type { AgentGuideStatusPayload } from "../bindings/AgentGuideStatusPayload";
import type { AgentId } from "../bindings/AgentId";
import { Button } from "./Button";
import type { AgentAccessActions } from "./settingsViewTypes";
import { AgentAccessConfirmationDialog } from "./AgentAccessConfirmationDialog";
import { StatusPill } from "./StatusPill";

const AGENTS: { id: AgentId; label: string }[] = [
  { id: "codex", label: "Codex" },
  { id: "claudeCode", label: "Claude Code" },
];

export function AgentAccessSettingsSection({ viewState, ...actions }: AgentAccessActions) {
  const statuses = viewState.statuses;
  const operating = viewState.busy !== null;
  const canInstallBoth = statuses?.some((status) => status.state === "notInstalled") ?? false;
  const operationLabel = viewState.busy === "both" ? "Updating both guides…"
    : viewState.busy === "codex" ? "Updating the Codex guide…"
      : viewState.busy === "claudeCode" ? "Updating the Claude Code guide…" : null;
  return (
    <section id="settings-agent-access" className="agent-access-settings">
      <div className="agent-access-intro">
        <div>
          <strong>Let local agents search completed notes when you ask.</strong>
          <p>Just Notes installs a read-only guide. It never sends transcripts or runs an agent itself.</p>
        </div>
        <Button
          variant="primary"
          size="compact"
          disabled={!canInstallBoth || operating}
          onClick={() => actions.onInstall(["codex", "claudeCode"])}
        >
          Install both
        </Button>
      </div>
      {operationLabel && <p className="agent-access-loading" role="status">
        <LoaderCircle size={14} aria-hidden="true" />{operationLabel}
      </p>}
      {viewState.error && <div className="settings-banner settings-banner-danger" role="alert">{viewState.error}</div>}
      {!statuses && viewState.busy === "loading" ? (
        <p className="agent-access-loading" role="status">
          <LoaderCircle size={14} aria-hidden="true" /> Reading guide folders…
        </p>
      ) : !statuses ? (
        <div className="agent-access-retry">
          <Button size="compact" onClick={actions.onRefresh}>Retry status check</Button>
        </div>
      ) : (
        <div className="agent-access-list">
          {AGENTS.map(({ id, label }) => (
            <AgentRow key={id} status={statuses.find((item) => item.agent === id)} label={label}
              operating={operating} onInstall={actions.onInstall} onRemove={actions.onRemove}
              onReveal={actions.onReveal} />
          ))}
        </div>
      )}
      {viewState.confirmation && (
        <AgentAccessConfirmationDialog confirmation={viewState.confirmation}
          onCancel={actions.onCancelConfirmation} onConfirm={actions.onConfirm} />
      )}
    </section>
  );
}

type RowProps = {
  status?: AgentGuideStatusPayload;
  label: string;
  operating: boolean;
  onInstall: (agents: AgentId[]) => void;
  onRemove: (agent: AgentId) => void;
  onReveal: (agent: AgentId) => void;
};

function AgentRow({ status, label, operating, onInstall, onRemove, onReveal }: RowProps) {
  if (!status) return null;
  const stateLabel = status.state === "installed" ? "Installed"
    : status.state === "conflict" ? "Conflict" : "Not installed";
  const tone = status.state === "installed" ? "ok" : status.state === "conflict" ? "warning" : "idle";
  return (
    <div className="agent-access-row">
      <div className="agent-access-row-copy">
        <strong className="agent-access-agent">
          {status.agent === "codex"
            ? <BsOpenai className="agent-access-brand-icon-codex" aria-hidden="true" />
            : <SiClaudecode className="agent-access-brand-icon-claude-code" aria-hidden="true" />}
          {label}
        </strong>
        <code className="settings-path">{status.path}</code>
        {status.detail && <p role={status.state === "conflict" ? undefined : "alert"}
          className={status.state === "conflict" ? "settings-hint-warning" : "settings-hint-danger"}>
          {status.detail}
        </p>}
      </div>
      <div className="settings-row-actions">
        <StatusPill tone={tone} label={stateLabel} />
        {status.state === "notInstalled" && <Button size="compact" disabled={operating}
          onClick={() => onInstall([status.agent])}>Install</Button>}
        {status.state === "installed" && <Button size="compact" variant="danger" disabled={operating}
          onClick={() => onRemove(status.agent)}>Remove</Button>}
        {status.state === "conflict" && <Button size="compact" disabled={operating}
          leadingIcon={<FolderOpen size={13} />} onClick={() => onReveal(status.agent)}>Reveal in Finder</Button>}
      </div>
    </div>
  );
}
