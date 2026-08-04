import { Bot, Trash2 } from "lucide-react";
import type { AgentAccessConfirmation } from "../features/app/useAgentAccessController";
import { Button } from "./Button";
import { ModalFrame } from "./ModalFrame";

const LABELS = { codex: "Codex", claudeCode: "Claude Code" } as const;

type Props = {
  confirmation: AgentAccessConfirmation;
  onCancel: () => void;
  onConfirm: () => void;
};

export function AgentAccessConfirmationDialog({ confirmation, onCancel, onConfirm }: Props) {
  const installing = confirmation.kind === "install";
  return (
    <ModalFrame
      role="alertdialog"
      className="modal-frame-wide"
      labelledBy="agent-access-confirmation-title"
      describedBy="agent-access-confirmation-description"
      icon={installing ? <Bot size={18} /> : <Trash2 size={18} />}
      onDismiss={onCancel}
      closeOnBackdrop
      actions={
        <>
          <Button onClick={onCancel}>Cancel</Button>
          <Button variant={installing ? "primary" : "danger"} onClick={onConfirm} autoFocus>
            {installing ? "Install" : "Remove and delete memory"}
          </Button>
        </>
      }
    >
      <h2 id="agent-access-confirmation-title">
        {installing ? "Install Agent Access?" : "Remove the final guide?"}
      </h2>
      <div id="agent-access-confirmation-description" className="agent-access-confirmation-copy">
        {installing ? (
          <>
            <p>An external agent may process transcript text when you ask it for note context.</p>
            <ul>
              {confirmation.destinations.map((destination) => (
                <li key={destination.agent}>
                  <strong>{LABELS[destination.agent]}</strong>
                  <code>{destination.path}</code>
                </li>
              ))}
            </ul>
          </>
        ) : (
          <p>Removing the last installed guide also permanently deletes shared Agent Access memory.</p>
        )}
      </div>
    </ModalFrame>
  );
}
