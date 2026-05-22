import { startWindowResize, type ResizeDirection } from "../lib/tauri";

const handles: Array<{ direction: ResizeDirection; className: string; label: string }> = [
  { direction: "North", className: "resize-handle--n", label: "向上调整窗口" },
  { direction: "South", className: "resize-handle--s", label: "向下调整窗口" },
  { direction: "East", className: "resize-handle--e", label: "向右调整窗口" },
  { direction: "West", className: "resize-handle--w", label: "向左调整窗口" },
  { direction: "NorthEast", className: "resize-handle--ne", label: "向右上调整窗口" },
  { direction: "NorthWest", className: "resize-handle--nw", label: "向左上调整窗口" },
  { direction: "SouthEast", className: "resize-handle--se", label: "向右下调整窗口" },
  { direction: "SouthWest", className: "resize-handle--sw", label: "向左下调整窗口" },
];

export function ResizeHandles() {
  return (
    <div className="resize-handles" aria-hidden="false">
      {handles.map((handle) => (
        <button
          key={handle.direction}
          className={`resize-handle ${handle.className}`}
          aria-label={handle.label}
          title={handle.label}
          onPointerDown={(event) => {
            event.preventDefault();
            event.stopPropagation();
            void startWindowResize(handle.direction);
          }}
        />
      ))}
    </div>
  );
}
