'use client';

import { useState, useRef, useEffect } from 'react';
import { ChatMessage } from '@/types/hydra';

interface ChatInterfaceProps {
  messages: ChatMessage[];
  onSend: (content: string) => void;
  disabled?: boolean;
  isProcessing?: boolean;
}

export function ChatInterface({ messages, onSend, disabled, isProcessing }: ChatInterfaceProps) {
  const [input, setInput] = useState('');
  const endRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (typeof endRef.current?.scrollIntoView === 'function') {
      endRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages.length]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = input.trim();
    if (!trimmed || disabled) return;
    onSend(trimmed);
    setInput('');
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Messages */}
      <div className="flex-1 overflow-y-auto custom-scrollbar px-4 py-6">
        <div className="max-w-2xl mx-auto space-y-4">
          {messages.length === 0 && (
            <div className="flex flex-col items-center justify-center h-full min-h-[300px] text-center">
              <div className="w-16 h-16 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 mb-4 opacity-30" />
              <h2 className="text-lg font-medium text-zinc-400 mb-1">Hydra</h2>
              <p className="text-sm text-zinc-600">Ask anything. I think before I act.</p>
            </div>
          )}

          {messages.map(msg => (
            <div
              key={msg.id}
              className={`flex message-enter ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
            >
              <div className={`max-w-[85%] ${msg.role === 'user' ? 'message-user' : 'message-hydra'}`}>
                <div className="px-4 py-2.5 text-sm leading-relaxed message-content">
                  <MessageContent content={msg.content} />
                </div>
                {msg.tokens_used !== undefined && msg.tokens_used > 0 && (
                  <div className="px-4 pb-1.5 text-[10px] text-zinc-500">
                    {msg.tokens_used} tokens
                  </div>
                )}
              </div>
            </div>
          ))}

          {/* Typing indicator */}
          {isProcessing && (
            <div className="flex justify-start message-enter">
              <div className="message-hydra px-4 py-3">
                <div className="typing-indicator flex gap-1">
                  <span />
                  <span />
                  <span />
                </div>
              </div>
            </div>
          )}

          <div ref={endRef} />
        </div>
      </div>

      {/* Input */}
      <div className="border-t border-zinc-800/50 p-4 bg-zinc-900/30 backdrop-blur-sm">
        <form onSubmit={handleSubmit} className="max-w-2xl mx-auto">
          <div className="flex gap-3 items-end">
            <input
              ref={inputRef}
              type="text"
              value={input}
              onChange={e => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask Hydra anything..."
              disabled={disabled}
              className="flex-1 chat-input disabled:opacity-40"
              aria-label="Message input"
            />
            <button
              type="submit"
              disabled={disabled || !input.trim()}
              className="px-5 py-3 rounded-xl text-sm font-medium transition-all duration-200
                         bg-gradient-to-r from-indigo-500 to-indigo-600
                         hover:from-indigo-400 hover:to-indigo-500
                         disabled:opacity-30 disabled:cursor-not-allowed
                         text-white shadow-lg shadow-indigo-500/20"
              aria-label="Send message"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <line x1="22" y1="2" x2="11" y2="13" />
                <polygon points="22 2 15 22 11 13 2 9 22 2" />
              </svg>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

/** Render message content with basic markdown: code blocks, inline code, bold */
function MessageContent({ content }: { content: string }) {
  // Split by code blocks
  const parts = content.split(/(```[\s\S]*?```)/g);

  return (
    <>
      {parts.map((part, i) => {
        if (part.startsWith('```') && part.endsWith('```')) {
          const inner = part.slice(3, -3);
          const newline = inner.indexOf('\n');
          const lang = newline > 0 ? inner.slice(0, newline).trim() : '';
          const code = newline > 0 ? inner.slice(newline + 1) : inner;
          return (
            <pre key={i} className="relative group">
              {lang && (
                <span className="absolute top-2 right-2 text-[10px] text-zinc-500 uppercase">{lang}</span>
              )}
              <code>{code}</code>
            </pre>
          );
        }

        // Handle inline code and bold
        return <InlineContent key={i} text={part} />;
      })}
    </>
  );
}

function InlineContent({ text }: { text: string }) {
  const parts = text.split(/(`[^`]+`|\*\*[^*]+\*\*)/g);
  return (
    <>
      {parts.map((part, i) => {
        if (part.startsWith('`') && part.endsWith('`')) {
          return <code key={i}>{part.slice(1, -1)}</code>;
        }
        if (part.startsWith('**') && part.endsWith('**')) {
          return <strong key={i} className="font-semibold text-white">{part.slice(2, -2)}</strong>;
        }
        return <span key={i}>{part}</span>;
      })}
    </>
  );
}
