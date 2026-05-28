import { useEffect, useRef, useState } from "react";

const SPRITE_PATH = "/avatars/star-lantern-cat-sprite-trimmed.png";

interface PixiNightCatAvatarProps {
  detail?: string;
  scale?: number;
}

type DisplayLike = {
  alpha: number;
  rotation: number;
  scale: {
    x: number;
    y: number;
    set: (value: number) => void;
  };
};

export function fitNightCatCanvasSize(rect: { width: number; height: number }) {
  return {
    width: Math.max(Math.round(rect.width), 1),
    height: Math.max(Math.round(rect.height), 1),
  };
}

function drawStar(graphics: any, cx: number, cy: number, outer: number, inner: number) {
  graphics.moveTo(cx, cy - outer);
  for (let i = 1; i < 10; i += 1) {
    const angle = -Math.PI / 2 + (i * Math.PI) / 5;
    const radius = i % 2 === 0 ? outer : inner;
    graphics.lineTo(cx + Math.cos(angle) * radius, cy + Math.sin(angle) * radius);
  }
  graphics.closePath();
}

function waitForTexture(texture: any) {
  if (texture.baseTexture.valid) return Promise.resolve();
  return new Promise<void>((resolve, reject) => {
    texture.baseTexture.once("loaded", () => resolve());
    texture.baseTexture.once("error", () => reject(new Error("Failed to load night cat sprite")));
  });
}

export function PixiNightCatAvatar({ detail, scale = 1 }: PixiNightCatAvatarProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const [isPixiReady, setIsPixiReady] = useState(false);
  const [hasPixiFailed, setHasPixiFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void) | undefined;
    setIsPixiReady(false);
    setHasPixiFailed(false);

    async function mount() {
      if (!hostRef.current) return;

      const PIXI = await import("pixi.js");
      if (cancelled || !hostRef.current) return;

      const host = hostRef.current;
      host.innerHTML = "";

      const app = new PIXI.Application({
        width: 1,
        height: 1,
        backgroundAlpha: 0,
        antialias: true,
        autoDensity: true,
        resolution: Math.min(window.devicePixelRatio || 1, 2),
      });

      const canvas = app.view as HTMLCanvasElement;
      canvas.setAttribute("data-tauri-drag-region", "true");
      host.appendChild(canvas);

      const texture = PIXI.Texture.from(SPRITE_PATH);
      await waitForTexture(texture);
      if (cancelled) {
        app.destroy(true, { children: true });
        return;
      }

      const root = new PIXI.Container();
      app.stage.addChild(root);

      const aura = new PIXI.Graphics() as typeof PIXI.Graphics.prototype & DisplayLike;
      aura.beginFill(0xf7b4d8, 0.16);
      aura.drawEllipse(0, -6, 120, 132);
      aura.endFill();
      aura.filters = [new PIXI.filters.BlurFilter(22)];
      root.addChild(aura);

      const shadow = new PIXI.Graphics();
      shadow.beginFill(0x26343a, 0.16);
      shadow.drawEllipse(0, 128, 86, 14);
      shadow.endFill();
      shadow.filters = [new PIXI.filters.BlurFilter(4)];
      root.addChild(shadow);

      const cat = new PIXI.Sprite(texture) as typeof PIXI.Sprite.prototype & DisplayLike;
      cat.anchor.set(0.5, 0.5);
      root.addChild(cat);

      const chestGlow = new PIXI.Graphics() as typeof PIXI.Graphics.prototype & DisplayLike;
      chestGlow.beginFill(0xffdc74, 0.32);
      chestGlow.drawCircle(0, 48, 32);
      chestGlow.endFill();
      chestGlow.filters = [new PIXI.filters.BlurFilter(10)];
      root.addChild(chestGlow);

      const sparkles: DisplayLike[] = [];
      for (const [x, y, size] of [
        [-116, -42, 5],
        [114, -82, 4],
        [96, 82, 6],
        [-82, 102, 4],
        [58, -138, 3],
      ]) {
        const sparkle = new PIXI.Graphics() as typeof PIXI.Graphics.prototype & DisplayLike;
        sparkle.beginFill(0xffdf85, 0.88);
        drawStar(sparkle, x, y, size, size * 0.42);
        sparkle.endFill();
        sparkle.filters = [new PIXI.filters.BlurFilter(0.5)];
        root.addChild(sparkle);
        sparkles.push(sparkle);
      }

      const fit = () => {
        const rect = host.getBoundingClientRect();
        const { width, height } = fitNightCatCanvasSize(rect);
        app.renderer.resize(width, height);
        root.x = width / 2;
        root.y = height / 2;

        const spriteWidth = texture.width;
        const spriteHeight = texture.height;
        const fitScale = Math.min(width / spriteWidth, height / spriteHeight) * 0.96 * scale;
        cat.scale.set(fitScale);
        root.scale.set(1);
      };

      fit();
      const observer = new ResizeObserver(fit);
      observer.observe(host);
      setIsPixiReady(true);
      setHasPixiFailed(false);

      let elapsed = 0;
      app.ticker.add((delta) => {
        elapsed += delta / 60;
        root.y = app.renderer.height / 2 + Math.sin(elapsed * 1.7) * -4;
        cat.rotation = Math.sin(elapsed * 1.1) * 0.012;
        cat.scale.y = cat.scale.x * (1 + Math.sin(elapsed * 1.45) * 0.012);
        aura.alpha = 0.48 + Math.sin(elapsed * 1.35) * 0.1;
        chestGlow.alpha = 0.58 + Math.sin(elapsed * 3) * 0.25;
        chestGlow.scale.set(0.92 + Math.sin(elapsed * 3) * 0.08);
        sparkles.forEach((sparkle, index) => {
          const wave = 0.5 + Math.sin(elapsed * 2.2 + index * 0.85) * 0.5;
          sparkle.alpha = 0.22 + wave * 0.64;
          sparkle.scale.set(0.82 + wave * 0.28);
        });
      });

      cleanup = () => {
        observer.disconnect();
        app.destroy(true, { children: true });
      };
    }

    void mount().catch(() => {
      if (!cancelled) {
        setIsPixiReady(false);
        setHasPixiFailed(true);
      }
    });

    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, [scale]);

  return (
    <div
      className={`pixi-night-cat-wrap${isPixiReady ? " is-pixi-ready" : ""}${
        hasPixiFailed ? " is-pixi-failed" : ""
      }`}
      data-tauri-drag-region
    >
      <img
        className="pixi-night-cat__fallback"
        src={SPRITE_PATH}
        alt="QPaw star lantern cat avatar"
        draggable={false}
        data-tauri-drag-region
      />
      <div ref={hostRef} className="pixi-night-cat" aria-label="QPaw star lantern cat avatar" data-tauri-drag-region />
      {detail ? <p className="avatar-status">{detail}</p> : null}
    </div>
  );
}
