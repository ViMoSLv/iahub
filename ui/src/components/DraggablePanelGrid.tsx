import { useState } from "react";
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  arrayMove,
  SortableContext,
  rectSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { TerminalPanel } from "./TerminalPanel";
import type { SessionInfo } from "../lib/types";

interface DraggablePanelGridProps {
  sessions: SessionInfo[];
  port: number;
  onReorder?: (sessions: SessionInfo[]) => void;
}

function SortablePanel({
  session,
  port,
}: {
  session: SessionInfo;
  port: number;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: session.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
    zIndex: isDragging ? 10 : undefined,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="h-full min-h-0"
      {...attributes}
    >
      {/* Drag handle — only the header bar is draggable */}
      <div
        {...listeners}
        className="cursor-grab active:cursor-grabbing"
        title="Drag to reorder"
      >
        <TerminalPanel session={session} port={port} isActive={!isDragging} />
      </div>
    </div>
  );
}

export function DraggablePanelGrid({
  sessions,
  port,
  onReorder,
}: DraggablePanelGridProps) {
  const [items, setItems] = useState(sessions);

  // Sync when sessions prop changes externally
  useState(() => {
    setItems(sessions);
  });

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    })
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const oldIndex = items.findIndex((s) => s.id === active.id);
    const newIndex = items.findIndex((s) => s.id === over.id);
    const reordered = arrayMove(items, oldIndex, newIndex);
    setItems(reordered);
    onReorder?.(reordered);
  };

  if (items.length === 0) {
    return (
      <div className="h-full flex items-center justify-center text-[#555] text-sm">
        Nenhuma sessão ativa — use + Nova Sessão ou Ctrl+K
      </div>
    );
  }

  const cols = items.length <= 2 ? 2 : items.length <= 4 ? 2 : 3;

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={handleDragEnd}
    >
      <SortableContext
        items={items.map((s) => s.id)}
        strategy={rectSortingStrategy}
      >
        <div
          className="h-full grid gap-[var(--panel-gap)]"
          style={{
            gridTemplateColumns: `repeat(${cols}, 1fr)`,
            gridTemplateRows: `repeat(${Math.ceil(items.length / cols)}, 1fr)`,
          }}
        >
          {items.map((session) => (
            <SortablePanel key={session.id} session={session} port={port} />
          ))}
        </div>
      </SortableContext>
    </DndContext>
  );
}