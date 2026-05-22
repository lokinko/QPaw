import { useEffect, useMemo, useRef, useState } from "react";
import { PlaceholderPet } from "./PlaceholderPet";
import { toAssetUrl } from "../lib/tauri";

interface Live2DAvatarProps {
  modelPath: string | null;
  scale: number;
}

type LoadState = "placeholder" | "loading" | "ready" | "runtime-missing" | "failed";

async function ensureCubismCoreLoaded() {
  if ("Live2DCubismCore" in window) return true;

  const runtimeUrl = "/vendor/live2dcubismcore.min.js";
  const response = await fetch(runtimeUrl, { method: "HEAD" }).catch(() => null);
  if (!response?.ok) return false;

  await new Promise<void>((resolve, reject) => {
    const script = document.createElement("script");
    script.src = runtimeUrl;
    script.async = true;
    script.onload = () => resolve();
    script.onerror = () => reject(new Error("Failed to load Live2D Cubism Core"));
    document.head.appendChild(script);
  });

  return "Live2DCubismCore" in window;
}

export function Live2DAvatar({ modelPath, scale }: Live2DAvatarProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const [state, setState] = useState<LoadState>("placeholder");
  const [detail, setDetail] = useState<string | undefined>();
  const assetUrl = useMemo(() => toAssetUrl(modelPath), [modelPath]);

  useEffect(() => {
    let cancelled = false;
    let appDestroy: (() => void) | undefined;

    async function loadModel() {
      if (!assetUrl || !hostRef.current) {
        setState("placeholder");
        setDetail(undefined);
        return;
      }

      setState("loading");
      setDetail("正在准备 Live2D 模型");

      try {
        const hasCore = await ensureCubismCoreLoaded();
        if (!hasCore) {
          setState("runtime-missing");
          setDetail("缺少官方 Cubism Core，已显示占位形象");
          return;
        }

        const PIXI = await import("pixi.js");
        const { Live2DModel } = await import("pixi-live2d-display/cubism4");
        if (cancelled || !hostRef.current) return;

        hostRef.current.innerHTML = "";
        const app = new PIXI.Application({
          width: 320,
          height: 360,
          transparent: true,
          antialias: true,
          autoDensity: true,
          resolution: Math.min(window.devicePixelRatio || 1, 2),
        });
        hostRef.current.appendChild(app.view as HTMLCanvasElement);

        const model = await Live2DModel.from(assetUrl);
        if (cancelled) {
          app.destroy(true);
          return;
        }

        model.scale.set(0.22 * scale);
        model.x = app.renderer.width / 2;
        model.y = app.renderer.height * 0.58;
        model.anchor?.set(0.5, 0.5);
        app.stage.addChild(model as never);
        appDestroy = () => app.destroy(true);
        setState("ready");
        setDetail(undefined);
      } catch (error) {
        console.error(error);
        setState("failed");
        setDetail("Live2D 加载失败，已回退到占位形象");
      }
    }

    void loadModel();

    return () => {
      cancelled = true;
      appDestroy?.();
    };
  }, [assetUrl, scale]);

  if (state === "ready" || state === "loading") {
    return (
      <div className="live2d-host-wrap" data-tauri-drag-region>
        <div ref={hostRef} className="live2d-host" data-tauri-drag-region />
        {state === "loading" ? <p className="avatar-status">{detail}</p> : null}
      </div>
    );
  }

  return <PlaceholderPet detail={detail} />;
}
