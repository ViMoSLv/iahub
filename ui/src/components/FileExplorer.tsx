import { useState, useCallback } from "react";

interface FileNode {
  name: string;
  path: string;
  isDirectory: boolean;
  children?: FileNode[];
  expanded?: boolean;
}

interface FileExplorerProps {
  rootFolder: FileNode | null;
  onFileSelect?: (path: string) => void;
  onClose?: () => void;
}

function FileIcon({ isDirectory, expanded }: { isDirectory: boolean; expanded?: boolean }) {
  if (isDirectory) {
    return (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" className="flex-shrink-0">
        <path
          d={expanded
            ? "M1.5 3h5l1.5 1.5h6.5v9h-13v-10.5z"
            : "M1.5 3h5l1.5 1.5h6.5v9h-13v-10.5z"
          }
          fill={expanded ? "#F0C24B" : "#C9A84C"}
          opacity={expanded ? 0.9 : 0.7}
        />
        {expanded && (
          <path d="M3 7l3 4 3-4H3z" fill="#0B0B0B" opacity={0.5} />
        )}
      </svg>
    );
  }
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" className="flex-shrink-0">
      <path d="M3 1.5h6.5L13 5v9.5H3V1.5z" fill="#2A2A2A" stroke="#555" strokeWidth="0.5" />
      <path d="M9.5 1.5V5H13" fill="#333" stroke="#555" strokeWidth="0.5" />
    </svg>
  );
}

function ChevronIcon({ expanded }: { expanded: boolean }) {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      fill="none"
      className={`flex-shrink-0 transition-transform duration-150 ${expanded ? "rotate-90" : ""}`}
    >
      <path d="M4 2l4 4-4 4" stroke="#7A7A7A" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function TreeNode({
  node,
  depth,
  onToggle,
  onSelect,
  selectedPath,
}: {
  node: FileNode;
  depth: number;
  onToggle: (path: string) => void;
  onSelect: (path: string) => void;
  selectedPath: string | null;
}) {
  const isSelected = selectedPath === node.path;
  const paddingLeft = 8 + depth * 16;

  return (
    <div>
      <div
        className={`flex items-center gap-1 py-[3px] pr-2 cursor-pointer text-[13px] transition-colors group ${
          isSelected
            ? "bg-[#1A1A1A] text-white"
            : "text-[#C9C9C9] hover:bg-[#161616]"
        }`}
        style={{ paddingLeft }}
        onClick={() => {
          if (node.isDirectory) {
            onToggle(node.path);
          } else {
            onSelect(node.path);
          }
        }}
      >
        {node.isDirectory ? (
          <ChevronIcon expanded={node.expanded || false} />
        ) : (
          <span className="w-3 flex-shrink-0" />
        )}
        <FileIcon isDirectory={node.isDirectory} expanded={node.expanded} />
        <span className="truncate ml-1">{node.name}</span>
      </div>
      {node.isDirectory && node.expanded && node.children && (
        <div className="tree-indent" style={{ marginLeft: paddingLeft + 6 }}>
          {node.children.map((child) => (
            <TreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              onToggle={onToggle}
              onSelect={onSelect}
              selectedPath={selectedPath}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export function FileExplorer({ rootFolder, onFileSelect, onClose }: FileExplorerProps) {
  const [tree, setTree] = useState<FileNode | null>(rootFolder);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  // Sync when rootFolder changes
  useState(() => {
    if (rootFolder) setTree(rootFolder);
  });

  const handleToggle = useCallback((path: string) => {
    setTree((prev) => {
      if (!prev) return prev;
      const toggle = (node: FileNode): FileNode => {
        if (node.path === path) {
          return { ...node, expanded: !node.expanded };
        }
        if (node.children) {
          return { ...node, children: node.children.map(toggle) };
        }
        return node;
      };
      return toggle(prev);
    });
  }, []);

  const handleSelect = useCallback((path: string) => {
    setSelectedPath(path);
    onFileSelect?.(path);
  }, [onFileSelect]);

  if (!tree) {
    return (
      <div className="h-full flex flex-col bg-[#0B0B0B]">
        <div className="h-9 flex items-center px-3 border-b border-[#171717] shrink-0">
          <span className="text-[11px] font-semibold uppercase tracking-wider text-[#7A7A7A]">
            Explorer
          </span>
          <div className="flex-1" />
          {onClose && (
            <button
              onClick={onClose}
              className="text-[#7A7A7A] hover:text-[#DCDCDC] text-xs p-0.5 rounded hover:bg-[#1A1A1A] transition-colors"
            >
              ✕
            </button>
          )}
        </div>
        <div className="flex-1 flex items-center justify-center p-4">
          <div className="text-center">
            <div className="text-[#7A7A7A] text-xs mb-2">Nenhuma pasta aberta</div>
            <div className="text-[#555] text-[11px]">
              Arraste uma pasta aqui para explorar os arquivos
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-[#0B0B0B] overflow-hidden">
      {/* Header */}
      <div className="h-9 flex items-center px-3 border-b border-[#171717] shrink-0">
        <span className="text-[11px] font-semibold uppercase tracking-wider text-[#7A7A7A]">
          Explorer
        </span>
        <div className="flex-1" />
        <span className="text-[10px] text-[#555] mr-2 truncate max-w-[100px]">
          {tree.name}
        </span>
        {onClose && (
          <button
            onClick={onClose}
            className="text-[#7A7A7A] hover:text-[#DCDCDC] text-xs p-0.5 rounded hover:bg-[#1A1A1A] transition-colors"
          >
            ✕
          </button>
        )}
      </div>

      {/* Tree */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden py-1">
        <TreeNode
          node={tree}
          depth={0}
          onToggle={handleToggle}
          onSelect={handleSelect}
          selectedPath={selectedPath}
        />
      </div>
    </div>
  );
}

// Helper to build a FileNode tree from a flat list of paths
export function buildFileTree(paths: string[], rootName: string): FileNode {
  const root: FileNode = {
    name: rootName,
    path: "",
    isDirectory: true,
    expanded: true,
    children: [],
  };

  for (const filePath of paths) {
    const parts = filePath.split(/[/\\]/).filter(Boolean);
    let current = root;

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      const childPath = parts.slice(0, i + 1).join("/");

      if (!current.children) current.children = [];

      let existing = current.children.find((c) => c.name === part);
      if (!existing) {
        existing = {
          name: part,
          path: childPath,
          isDirectory: !isLast,
          expanded: !isLast && i < 2, // auto-expand first 2 levels
          children: !isLast ? [] : undefined,
        };
        current.children.push(existing);
      }
      current = existing;
    }
  }

  // Sort: directories first, then alphabetical
  const sortChildren = (node: FileNode) => {
    if (node.children) {
      node.children.sort((a, b) => {
        if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
        return a.name.localeCompare(b.name);
      });
      node.children.forEach(sortChildren);
    }
  };
  sortChildren(root);

  return root;
}