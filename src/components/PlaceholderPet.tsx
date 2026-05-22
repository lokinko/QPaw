import { Bot } from "lucide-react";

interface PlaceholderPetProps {
  detail?: string;
}

export function PlaceholderPet({ detail }: PlaceholderPetProps) {
  return (
    <div className="placeholder-pet" aria-label="QPaw placeholder avatar" data-tauri-drag-region>
      <div className="placeholder-pet__halo" />
      <div className="placeholder-pet__body" data-tauri-drag-region>
        <div className="placeholder-pet__ears" data-tauri-drag-region>
          <span />
          <span />
        </div>
        <div className="placeholder-pet__face" data-tauri-drag-region>
          <span className="placeholder-pet__eye" />
          <Bot size={34} strokeWidth={1.8} />
          <span className="placeholder-pet__eye" />
        </div>
        <div className="placeholder-pet__base" />
      </div>
      {detail ? <p className="avatar-status">{detail}</p> : null}
    </div>
  );
}
