import React, { useState, useEffect, useRef, useCallback } from "react";
import { Message, Session } from "@workspace/api-client-react";

export function TypewriterText({ text, speed = 10 }: { text: string; speed?: number }) {
  const [displayed, setDisplayed] = useState("");
  
  useEffect(() => {
    let i = 0;
    const interval = setInterval(() => {
      setDisplayed(text.slice(0, i));
      i++;
      if (i > text.length) clearInterval(interval);
    }, speed);
    return () => clearInterval(interval);
  }, [text, speed]);

  return <span>{displayed}</span>;
}

// Markdown renderer could be added, but for now we'll just format basic code blocks.
export function FormattedMessage({ content }: { content: string }) {
  // Split by code blocks
  const parts = content.split(/(```[\s\S]*?```)/g);
  
  return (
    <div className="space-y-4">
      {parts.map((part, i) => {
        if (part.startsWith("```") && part.endsWith("```")) {
          const code = part.slice(3, -3);
          const firstLineBreak = code.indexOf("\n");
          const language = firstLineBreak > -1 ? code.slice(0, firstLineBreak).trim() : "";
          const codeContent = firstLineBreak > -1 ? code.slice(firstLineBreak + 1) : code;
          
          return (
            <div key={i} className="rounded-md bg-[#0a0a0c] border border-border overflow-hidden">
              {language && (
                <div className="bg-[#141416] px-3 py-1 text-xs text-muted-foreground border-b border-border flex justify-between items-center">
                  <span>{language}</span>
                </div>
              )}
              <pre className="p-3 text-sm font-mono overflow-x-auto text-[#e2e8f0]">
                <code>{codeContent}</code>
              </pre>
            </div>
          );
        }
        return <div key={i} className="whitespace-pre-wrap">{part}</div>;
      })}
    </div>
  );
}
