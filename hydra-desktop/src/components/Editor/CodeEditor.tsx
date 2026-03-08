'use client';

import { useRef, useCallback } from 'react';
import dynamic from 'next/dynamic';

const MonacoEditor = dynamic(() => import('@monaco-editor/react').then(m => m.default), {
  ssr: false,
  loading: () => <div className="flex items-center justify-center h-full text-gray-500">Loading editor...</div>,
});

interface CodeEditorProps {
  value: string;
  language?: string;
  readOnly?: boolean;
  onChange?: (value: string) => void;
}

export function CodeEditor({
  value,
  language = 'typescript',
  readOnly = false,
  onChange,
}: CodeEditorProps) {
  const editorRef = useRef<unknown>(null);

  const handleMount = useCallback((editor: unknown) => {
    editorRef.current = editor;
  }, []);

  return (
    <div className="h-full w-full" data-testid="code-editor">
      <MonacoEditor
        height="100%"
        language={language}
        value={value}
        theme="vs-dark"
        onChange={(val) => onChange?.(val ?? '')}
        onMount={handleMount}
        options={{
          readOnly,
          minimap: { enabled: false },
          fontSize: 13,
          lineNumbers: 'on',
          scrollBeyondLastLine: false,
          wordWrap: 'on',
          padding: { top: 8 },
        }}
      />
    </div>
  );
}
