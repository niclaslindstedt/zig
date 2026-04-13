import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ChatEvent,
  sendChatMessage,
  startChat,
  streamChat,
} from "./api";

interface Message {
  id: string;
  role: "user" | "agent" | "system";
  text: string;
}

type Stage = "empty" | "chat";

const uid = () => Math.random().toString(36).slice(2, 10);

export default function App() {
  const [stage, setStage] = useState<Stage>("empty");
  const [draft, setDraft] = useState("");
  const [messages, setMessages] = useState<Message[]>([]);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [outputPath, setOutputPath] = useState<string | null>(null);
  const [isSending, setIsSending] = useState(false);
  const [agentTyping, setAgentTyping] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const scrollRef = useRef<HTMLDivElement | null>(null);
  const closeStreamRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    return () => {
      closeStreamRef.current?.();
    };
  }, []);

  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [messages, agentTyping]);

  const appendMessage = useCallback((msg: Omit<Message, "id">) => {
    setMessages((prev) => [...prev, { ...msg, id: uid() }]);
  }, []);

  const handleChatEvent = useCallback(
    (event: ChatEvent) => {
      if (event.role === "agent") setAgentTyping(false);
      appendMessage({ role: event.role, text: event.text });
    },
    [appendMessage],
  );

  const handleCreate = useCallback(async () => {
    const prompt = draft.trim();
    if (!prompt || isSending) return;
    setError(null);
    setIsSending(true);
    setAgentTyping(true);
    try {
      const res = await startChat(prompt);
      setSessionId(res.session_id);
      setOutputPath(res.output_path);
      appendMessage({ role: "user", text: prompt });
      setDraft("");
      setStage("chat");
      closeStreamRef.current?.();
      closeStreamRef.current = streamChat(
        res.session_id,
        handleChatEvent,
        () => setAgentTyping(false),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setAgentTyping(false);
    } finally {
      setIsSending(false);
    }
  }, [appendMessage, draft, handleChatEvent, isSending]);

  const handleSend = useCallback(async () => {
    const text = draft.trim();
    if (!text || !sessionId || isSending) return;
    setError(null);
    setIsSending(true);
    setAgentTyping(true);
    try {
      appendMessage({ role: "user", text });
      setDraft("");
      await sendChatMessage(sessionId, text);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setAgentTyping(false);
    } finally {
      setIsSending(false);
    }
  }, [appendMessage, draft, isSending, sessionId]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        if (stage === "empty") {
          void handleCreate();
        } else {
          void handleSend();
        }
      }
    },
    [handleCreate, handleSend, stage],
  );

  const handleNew = useCallback(() => {
    closeStreamRef.current?.();
    closeStreamRef.current = null;
    setStage("empty");
    setMessages([]);
    setSessionId(null);
    setOutputPath(null);
    setDraft("");
    setError(null);
    setAgentTyping(false);
  }, []);

  const disabled = !draft.trim() || isSending;

  if (stage === "empty") {
    return (
      <EmptyState
        draft={draft}
        onDraftChange={setDraft}
        onSubmit={handleCreate}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        error={error}
      />
    );
  }

  return (
    <ChatView
      messages={messages}
      draft={draft}
      onDraftChange={setDraft}
      onSend={handleSend}
      onKeyDown={handleKeyDown}
      onNew={handleNew}
      disabled={disabled}
      agentTyping={agentTyping}
      outputPath={outputPath}
      scrollRef={scrollRef}
      error={error}
    />
  );
}

interface EmptyStateProps {
  draft: string;
  onDraftChange: (v: string) => void;
  onSubmit: () => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  disabled: boolean;
  error: string | null;
}

function EmptyState({
  draft,
  onDraftChange,
  onSubmit,
  onKeyDown,
  disabled,
  error,
}: EmptyStateProps) {
  return (
    <div className="flex h-full w-full items-center justify-center p-6">
      <div className="w-full max-w-2xl">
        <div className="mb-8 text-center">
          <div className="mb-3 text-5xl">🔀</div>
          <h1 className="text-3xl font-semibold tracking-tight text-[var(--color-text-primary)]">
            Create a workflow
          </h1>
          <p className="mt-2 text-[var(--color-text-secondary)]">
            Describe the process you want to automate. An agent will help you
            design and save a <code className="rounded bg-[var(--color-surface-alt)] px-1.5 py-0.5 font-mono text-sm">.zug</code>{" "}
            file.
          </p>
        </div>
        <div className="rounded-2xl border border-[var(--color-border)] bg-[var(--color-surface-alt)]/80 p-4 shadow-2xl shadow-black/40 backdrop-blur-sm focus-within:border-[var(--color-accent)] focus-within:ring-2 focus-within:ring-[var(--color-accent)]/30 transition">
          <textarea
            value={draft}
            onChange={(e) => onDraftChange(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="e.g. Review a pull request, run the test suite, and summarize the findings..."
            rows={5}
            autoFocus
            className="w-full resize-none bg-transparent text-base leading-relaxed text-[var(--color-text-primary)] placeholder:text-[var(--color-text-dim)] outline-none"
          />
          <div className="mt-3 flex items-center justify-between">
            <span className="text-xs text-[var(--color-text-dim)]">
              Press Enter to create · Shift+Enter for newline
            </span>
            <button
              type="button"
              onClick={onSubmit}
              disabled={disabled}
              className="rounded-xl bg-[var(--color-accent)] px-5 py-2 font-medium text-white shadow-lg shadow-[var(--color-accent)]/20 transition hover:bg-[var(--color-accent-light)] disabled:cursor-not-allowed disabled:opacity-40"
            >
              Create
            </button>
          </div>
        </div>
        {error && (
          <div className="mt-4 rounded-lg border border-red-500/40 bg-red-500/10 p-3 text-sm text-red-300">
            {error}
          </div>
        )}
      </div>
    </div>
  );
}

interface ChatViewProps {
  messages: Message[];
  draft: string;
  onDraftChange: (v: string) => void;
  onSend: () => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  onNew: () => void;
  disabled: boolean;
  agentTyping: boolean;
  outputPath: string | null;
  scrollRef: React.RefObject<HTMLDivElement | null>;
  error: string | null;
}

function ChatView({
  messages,
  draft,
  onDraftChange,
  onSend,
  onKeyDown,
  onNew,
  disabled,
  agentTyping,
  outputPath,
  scrollRef,
  error,
}: ChatViewProps) {
  const subtitle = useMemo(() => {
    if (outputPath) return outputPath;
    return "workflow creation session";
  }, [outputPath]);

  return (
    <div className="flex h-full w-full flex-col">
      <header className="flex items-center justify-between border-b border-[var(--color-border)] bg-[var(--color-surface-alt)]/60 px-6 py-4 backdrop-blur-sm">
        <div className="flex items-center gap-3">
          <div className="text-2xl">🔀</div>
          <div>
            <div className="text-sm font-semibold text-[var(--color-text-primary)]">
              zig workflow chat
            </div>
            <div className="font-mono text-xs text-[var(--color-text-dim)]">
              {subtitle}
            </div>
          </div>
        </div>
        <button
          type="button"
          onClick={onNew}
          className="rounded-lg border border-[var(--color-border)] bg-[var(--color-surface-hover)] px-3 py-1.5 text-sm text-[var(--color-text-secondary)] transition hover:text-[var(--color-text-primary)]"
        >
          New
        </button>
      </header>

      <div
        ref={scrollRef}
        className="chat-scroll flex-1 overflow-y-auto px-4 py-8 sm:px-8"
      >
        <div className="mx-auto flex max-w-3xl flex-col gap-4">
          {messages.map((m) => (
            <MessageBubble key={m.id} message={m} />
          ))}
          {agentTyping && <TypingIndicator />}
        </div>
      </div>

      {error && (
        <div className="border-t border-red-500/40 bg-red-500/10 px-6 py-2 text-sm text-red-300">
          {error}
        </div>
      )}

      <footer className="border-t border-[var(--color-border)] bg-[var(--color-surface-alt)]/70 px-4 py-4 sm:px-8 backdrop-blur-sm">
        <div className="mx-auto flex max-w-3xl items-end gap-3 rounded-2xl border border-[var(--color-border)] bg-[var(--color-surface)] p-3 focus-within:border-[var(--color-accent)] focus-within:ring-2 focus-within:ring-[var(--color-accent)]/30 transition">
          <textarea
            value={draft}
            onChange={(e) => onDraftChange(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="Reply to the agent..."
            rows={1}
            autoFocus
            className="max-h-40 flex-1 resize-none bg-transparent text-sm leading-relaxed text-[var(--color-text-primary)] placeholder:text-[var(--color-text-dim)] outline-none"
          />
          <button
            type="button"
            onClick={onSend}
            disabled={disabled}
            className="rounded-xl bg-[var(--color-accent)] px-4 py-2 text-sm font-medium text-white shadow-md shadow-[var(--color-accent)]/20 transition hover:bg-[var(--color-accent-light)] disabled:cursor-not-allowed disabled:opacity-40"
          >
            Send
          </button>
        </div>
      </footer>
    </div>
  );
}

function MessageBubble({ message }: { message: Message }) {
  if (message.role === "system") {
    return (
      <div className="self-center rounded-full border border-[var(--color-border)] bg-[var(--color-surface-alt)] px-3 py-1 font-mono text-xs text-[var(--color-text-dim)]">
        {message.text}
      </div>
    );
  }
  const isUser = message.role === "user";
  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[80%] whitespace-pre-wrap rounded-2xl px-4 py-3 text-sm leading-relaxed shadow-lg ${
          isUser
            ? "rounded-br-md bg-[var(--color-accent)]/90 text-white shadow-[var(--color-accent)]/20"
            : "rounded-bl-md border border-[var(--color-border)] bg-[var(--color-surface-alt)] text-[var(--color-text-primary)] shadow-black/30"
        }`}
      >
        {message.text}
      </div>
    </div>
  );
}

function TypingIndicator() {
  return (
    <div className="flex justify-start">
      <div className="flex items-center gap-1.5 rounded-2xl rounded-bl-md border border-[var(--color-border)] bg-[var(--color-surface-alt)] px-4 py-3">
        <span className="typing-dot h-1.5 w-1.5 rounded-full bg-[var(--color-text-secondary)]" />
        <span className="typing-dot h-1.5 w-1.5 rounded-full bg-[var(--color-text-secondary)]" />
        <span className="typing-dot h-1.5 w-1.5 rounded-full bg-[var(--color-text-secondary)]" />
      </div>
    </div>
  );
}
