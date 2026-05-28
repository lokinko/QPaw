import { type CSSProperties, useEffect, useMemo, useState } from "react";
import { PlaceholderPet } from "./PlaceholderPet";
import { toAssetUrl } from "../lib/tauri";

interface StaticAvatarProps {
  imagePath: string | null;
  scale: number;
}

export function StaticAvatar({ imagePath, scale }: StaticAvatarProps) {
  const [failed, setFailed] = useState(false);
  const assetUrl = useMemo(() => toAssetUrl(imagePath), [imagePath]);
  const style = { "--avatar-scale": String(scale) } as CSSProperties;

  useEffect(() => {
    setFailed(false);
  }, [assetUrl]);

  if (!assetUrl || failed) {
    return <PlaceholderPet detail={failed ? "图片加载失败，已回退到占位形象" : undefined} />;
  }

  return (
    <div className="static-avatar-wrap" style={style} data-tauri-drag-region>
      <img
        className="static-avatar"
        src={assetUrl}
        alt="QPaw avatar"
        draggable={false}
        data-tauri-drag-region
        onError={() => setFailed(true)}
      />
    </div>
  );
}
