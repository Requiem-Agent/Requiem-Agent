// chat-history-sidebar.tsx — S8-05: Chat History Sidebar
// شريط جانبي يعرض المحادثات السابقة مع البحث والتصفية

import { useState, useCallback, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

interface Conversation {
  id: string;
  title: string;
  model: string;
  provider: string;
  mode: "chat" | "orchestrator" | "code";
  message_count: number;
  total_tokens: number;
  is_pinned: boolean;
  last_message_at: string | null;
  created_at: string;
  last_user_message: string | null;
}

interface ConversationsResponse {
  success: boolean;
  data: {
    conversations: Conversation[];
    total: number;
    page: number;
    per_page: number;
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// API hooks
// ─────────────────────────────────────────────────────────────────────────────

const API_BASE = import.meta.env.VITE_API_URL ?? "http://localhost:7860";

async function fetchConversations(search?: string, page = 1): Promise<ConversationsResponse> {
  const params = new URLSearchParams({ page: String(page), per_page: "20" });
  if (search) params.set("search", search);

  const res = await fetch(`${API_BASE}/api/conversations?${params}`, {
    headers: { "Content-Type": "application/json" },
    credentials: "include",
  });
  if (!res.ok) throw new Error(`Failed to fetch conversations: ${res.status}`);
  return res.json();
}

async function deleteConversation(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/api/conversations/${id}`, {
    method: "DELETE",
    credentials: "include",
  });
  if (!res.ok) throw new Error(`Failed to delete conversation: ${res.status}`);
}

async function pinConversation(id: string, pinned: boolean): Promise<void> {
  const res = await fetch(`${API_BASE}/api/conversations/${id}/pin`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ is_pinned: pinned }),
    credentials: "include",
  });
  if (!res.ok) throw new Error(`Failed to pin conversation: ${res.status}`);
}

function useConversations(search: string, page: number) {
  return useQuery({
    queryKey: ["conversations", search, page],
    queryFn: () => fetchConversations(search || undefined, page),
    staleTime: 30_000,
    placeholderData: (prev) => prev,
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

function formatRelativeTime(dateStr: string | null): string {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60_000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "الآن";
  if (diffMins < 60) return `منذ ${diffMins} دقيقة`;
  if (diffHours < 24) return `منذ ${diffHours} ساعة`;
  if (diffDays < 7) return `منذ ${diffDays} يوم`;
  return date.toLocaleDateString("ar-DZ");
}

function getModeIcon(mode: string): string {
  switch (mode) {
    case "orchestrator": return "🤖";
    case "code": return "💻";
    default: return "💬";
  }
}

function getProviderColor(provider: string): string {
  switch (provider) {
    case "anthropic": return "#d97706";
    case "openai": return "#10b981";
    case "gemini": return "#3b82f6";
    case "mistral": return "#8b5cf6";
    default: return "#6b7280";
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// ConversationItem component
// ─────────────────────────────────────────────────────────────────────────────

interface ConversationItemProps {
  conversation: Conversation;
  isActive: boolean;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
  onPin: (id: string, pinned: boolean) => void;
}

function ConversationItem({
  conversation: conv,
  isActive,
  onSelect,
  onDelete,
  onPin,
}: ConversationItemProps) {
  const [showMenu, setShowMenu] = useState(false);

  return (
    <div
      className={`conv-item ${isActive ? "active" : ""}`}
      onClick={() => onSelect(conv.id)}
      onMouseLeave={() => setShowMenu(false)}
    >
      <div className="conv-item-header">
        <span className="conv-mode-icon">{getModeIcon(conv.mode)}</span>
        <span className="conv-title">{conv.title}</span>
        <div className="conv-actions">
          {conv.is_pinned && <span className="pin-indicator">📌</span>}
          <button
            className="menu-btn"
            onClick={(e) => {
              e.stopPropagation();
              setShowMenu(!showMenu);
            }}
          >
            ⋮
          </button>
        </div>
      </div>

      {conv.last_user_message && (
        <p className="conv-preview">{conv.last_user_message}</p>
      )}

      <div className="conv-meta">
        <span
          className="conv-provider"
          style={{ color: getProviderColor(conv.provider) }}
        >
          {conv.provider}
        </span>
        <span className="conv-count">{conv.message_count} رسالة</span>
        <span className="conv-time">
          {formatRelativeTime(conv.last_message_at ?? conv.created_at)}
        </span>
      </div>

      {showMenu && (
        <div className="conv-menu" onClick={(e) => e.stopPropagation()}>
          <button
            onClick={() => {
              onPin(conv.id, !conv.is_pinned);
              setShowMenu(false);
            }}
          >
            {conv.is_pinned ? "📌 إلغاء التثبيت" : "📌 تثبيت"}
          </button>
          <button
            className="danger"
            onClick={() => {
              onDelete(conv.id);
              setShowMenu(false);
            }}
          >
            🗑️ حذف
          </button>
        </div>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Main sidebar component
// ─────────────────────────────────────────────────────────────────────────────

interface ChatHistorySidebarProps {
  activeConversationId?: string;
  onSelectConversation: (id: string) => void;
  onNewConversation: () => void;
  isCollapsed?: boolean;
  onToggleCollapse?: () => void;
}

export default function ChatHistorySidebar({
  activeConversationId,
  onSelectConversation,
  onNewConversation,
  isCollapsed = false,
  onToggleCollapse,
}: ChatHistorySidebarProps) {
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [page, setPage] = useState(1);
  const queryClient = useQueryClient();

  // Debounce البحث
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(search);
      setPage(1);
    }, 300);
    return () => clearTimeout(timer);
  }, [search]);

  const { data, isLoading, error } = useConversations(debouncedSearch, page);

  const deleteMutation = useMutation({
    mutationFn: deleteConversation,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
    },
  });

  const pinMutation = useMutation({
    mutationFn: ({ id, pinned }: { id: string; pinned: boolean }) =>
      pinConversation(id, pinned),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
    },
  });

  const handleDelete = useCallback(
    (id: string) => {
      if (confirm("هل تريد حذف هذه المحادثة؟")) {
        deleteMutation.mutate(id);
      }
    },
    [deleteMutation]
  );

  const handlePin = useCallback(
    (id: string, pinned: boolean) => {
      pinMutation.mutate({ id, pinned });
    },
    [pinMutation]
  );

  const conversations = data?.data.conversations ?? [];
  const pinned = conversations.filter((c) => c.is_pinned);
  const unpinned = conversations.filter((c) => !c.is_pinned);
  const total = data?.data.total ?? 0;
  const hasMore = conversations.length < total;

  if (isCollapsed) {
    return (
      <div className="sidebar collapsed">
        <button className="collapse-btn" onClick={onToggleCollapse} title="توسيع">
          ▶
        </button>
        <button className="new-chat-icon" onClick={onNewConversation} title="محادثة جديدة">
          ✏️
        </button>
      </div>
    );
  }

  return (
    <div className="chat-sidebar" dir="rtl">
      {/* Header */}
      <div className="sidebar-header">
        <h2 className="sidebar-title">المحادثات</h2>
        <div className="sidebar-header-actions">
          <button className="new-chat-btn" onClick={onNewConversation} title="محادثة جديدة">
            ✏️ جديد
          </button>
          {onToggleCollapse && (
            <button className="collapse-btn" onClick={onToggleCollapse} title="طيّ">
              ◀
            </button>
          )}
        </div>
      </div>

      {/* Search */}
      <div className="search-container">
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="🔍 بحث في المحادثات..."
          className="search-input"
        />
        {search && (
          <button className="clear-search" onClick={() => setSearch("")}>
            ✕
          </button>
        )}
      </div>

      {/* Content */}
      <div className="conversations-list">
        {isLoading && (
          <div className="loading-state">
            <div className="spinner-sm" />
            <span>جارٍ التحميل...</span>
          </div>
        )}

        {error && (
          <div className="error-state">
            ⚠️ فشل تحميل المحادثات
          </div>
        )}

        {!isLoading && conversations.length === 0 && (
          <div className="empty-state">
            <span className="empty-icon">💬</span>
            <p>{search ? "لا توجد نتائج" : "لا توجد محادثات بعد"}</p>
            {!search && (
              <button className="start-btn" onClick={onNewConversation}>
                ابدأ محادثة جديدة
              </button>
            )}
          </div>
        )}

        {/* Pinned conversations */}
        {pinned.length > 0 && (
          <div className="conv-group">
            <div className="group-label">📌 مثبّتة</div>
            {pinned.map((conv) => (
              <ConversationItem
                key={conv.id}
                conversation={conv}
                isActive={conv.id === activeConversationId}
                onSelect={onSelectConversation}
                onDelete={handleDelete}
                onPin={handlePin}
              />
            ))}
          </div>
        )}

        {/* Recent conversations */}
        {unpinned.length > 0 && (
          <div className="conv-group">
            {pinned.length > 0 && <div className="group-label">🕐 الأخيرة</div>}
            {unpinned.map((conv) => (
              <ConversationItem
                key={conv.id}
                conversation={conv}
                isActive={conv.id === activeConversationId}
                onSelect={onSelectConversation}
                onDelete={handleDelete}
                onPin={handlePin}
              />
            ))}
          </div>
        )}

        {/* Load more */}
        {hasMore && (
          <button
            className="load-more-btn"
            onClick={() => setPage((p) => p + 1)}
          >
            تحميل المزيد ({total - conversations.length} متبقية)
          </button>
        )}
      </div>

      {/* Stats footer */}
      {total > 0 && (
        <div className="sidebar-footer">
          <span>{total} محادثة</span>
        </div>
      )}

      <style>{`
        .chat-sidebar {
          width: 280px;
          min-width: 280px;
          height: 100%;
          background: #0f172a;
          border-left: 1px solid #1e293b;
          display: flex;
          flex-direction: column;
          overflow: hidden;
          font-family: 'Segoe UI', Tahoma, sans-serif;
        }
        .sidebar.collapsed {
          width: 48px;
          min-width: 48px;
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 0.5rem 0;
          gap: 0.5rem;
          background: #0f172a;
          border-left: 1px solid #1e293b;
        }
        .sidebar-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 1rem;
          border-bottom: 1px solid #1e293b;
        }
        .sidebar-title { font-size: 1rem; font-weight: 600; color: #e2e8f0; margin: 0; }
        .sidebar-header-actions { display: flex; gap: 0.5rem; }
        .new-chat-btn, .collapse-btn, .new-chat-icon {
          background: #1e293b;
          border: none;
          color: #94a3b8;
          padding: 0.35rem 0.6rem;
          border-radius: 6px;
          cursor: pointer;
          font-size: 0.8rem;
          transition: all 0.2s;
        }
        .new-chat-btn:hover, .collapse-btn:hover, .new-chat-icon:hover {
          background: #334155;
          color: #e2e8f0;
        }
        .search-container {
          position: relative;
          padding: 0.75rem;
          border-bottom: 1px solid #1e293b;
        }
        .search-input {
          width: 100%;
          background: #1e293b;
          border: 1px solid #334155;
          border-radius: 8px;
          padding: 0.5rem 2rem 0.5rem 0.75rem;
          color: #e2e8f0;
          font-size: 0.85rem;
          outline: none;
          box-sizing: border-box;
          text-align: right;
        }
        .search-input:focus { border-color: #3b82f6; }
        .search-input::placeholder { color: #475569; }
        .clear-search {
          position: absolute;
          left: 1.25rem;
          top: 50%;
          transform: translateY(-50%);
          background: none;
          border: none;
          color: #64748b;
          cursor: pointer;
          font-size: 0.75rem;
          padding: 0.2rem;
        }
        .conversations-list {
          flex: 1;
          overflow-y: auto;
          padding: 0.5rem 0;
        }
        .conversations-list::-webkit-scrollbar { width: 4px; }
        .conversations-list::-webkit-scrollbar-track { background: transparent; }
        .conversations-list::-webkit-scrollbar-thumb { background: #334155; border-radius: 2px; }
        .conv-group { margin-bottom: 0.5rem; }
        .group-label {
          font-size: 0.7rem;
          color: #64748b;
          padding: 0.25rem 1rem;
          text-transform: uppercase;
          letter-spacing: 0.05em;
        }
        .conv-item {
          position: relative;
          padding: 0.75rem 1rem;
          cursor: pointer;
          border-radius: 0;
          transition: background 0.15s;
          border-right: 3px solid transparent;
        }
        .conv-item:hover { background: #1e293b; }
        .conv-item.active {
          background: #1e293b;
          border-right-color: #3b82f6;
        }
        .conv-item-header {
          display: flex;
          align-items: center;
          gap: 0.4rem;
          margin-bottom: 0.25rem;
        }
        .conv-mode-icon { font-size: 0.9rem; flex-shrink: 0; }
        .conv-title {
          flex: 1;
          font-size: 0.875rem;
          font-weight: 500;
          color: #e2e8f0;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }
        .conv-actions {
          display: flex;
          align-items: center;
          gap: 0.25rem;
          opacity: 0;
          transition: opacity 0.15s;
        }
        .conv-item:hover .conv-actions { opacity: 1; }
        .pin-indicator { font-size: 0.7rem; }
        .menu-btn {
          background: none;
          border: none;
          color: #94a3b8;
          cursor: pointer;
          padding: 0.1rem 0.3rem;
          border-radius: 4px;
          font-size: 1rem;
          line-height: 1;
        }
        .menu-btn:hover { background: #334155; }
        .conv-preview {
          font-size: 0.75rem;
          color: #64748b;
          margin: 0 0 0.25rem;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }
        .conv-meta {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          font-size: 0.7rem;
        }
        .conv-provider { font-weight: 500; }
        .conv-count { color: #475569; }
        .conv-time { color: #475569; margin-right: auto; }
        .conv-menu {
          position: absolute;
          left: 0.5rem;
          top: 100%;
          background: #1e293b;
          border: 1px solid #334155;
          border-radius: 8px;
          padding: 0.25rem;
          z-index: 100;
          min-width: 140px;
          box-shadow: 0 4px 12px rgba(0,0,0,0.4);
        }
        .conv-menu button {
          display: block;
          width: 100%;
          background: none;
          border: none;
          color: #e2e8f0;
          padding: 0.5rem 0.75rem;
          text-align: right;
          cursor: pointer;
          border-radius: 6px;
          font-size: 0.8rem;
          transition: background 0.15s;
        }
        .conv-menu button:hover { background: #334155; }
        .conv-menu button.danger { color: #f87171; }
        .conv-menu button.danger:hover { background: rgba(239,68,68,0.1); }
        .loading-state, .error-state, .empty-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 2rem 1rem;
          gap: 0.5rem;
          color: #64748b;
          font-size: 0.875rem;
          text-align: center;
        }
        .empty-icon { font-size: 2rem; }
        .start-btn {
          background: #3b82f6;
          color: white;
          border: none;
          padding: 0.5rem 1rem;
          border-radius: 8px;
          cursor: pointer;
          font-size: 0.8rem;
          margin-top: 0.5rem;
        }
        .spinner-sm {
          width: 20px; height: 20px;
          border: 2px solid #334155;
          border-top-color: #3b82f6;
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
        }
        @keyframes spin { to { transform: rotate(360deg); } }
        .load-more-btn {
          display: block;
          width: calc(100% - 2rem);
          margin: 0.5rem 1rem;
          background: #1e293b;
          border: 1px solid #334155;
          color: #94a3b8;
          padding: 0.5rem;
          border-radius: 8px;
          cursor: pointer;
          font-size: 0.8rem;
          transition: all 0.2s;
        }
        .load-more-btn:hover { background: #334155; color: #e2e8f0; }
        .sidebar-footer {
          padding: 0.5rem 1rem;
          border-top: 1px solid #1e293b;
          font-size: 0.75rem;
          color: #475569;
          text-align: center;
        }
      `}</style>
    </div>
  );
}
