declare module "pixi-live2d-display/cubism4" {
  import type { DisplayObject } from "pixi.js";

  export class Live2DModel extends DisplayObject {
    static from(source: string): Promise<Live2DModel>;
    scale: { set(value: number): void };
    anchor?: { set(x: number, y?: number): void };
    x: number;
    y: number;
    width: number;
    height: number;
  }
}
