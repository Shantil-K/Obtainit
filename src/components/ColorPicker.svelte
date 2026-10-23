<script lang="ts">
  // Full-spectrum color picker: saturation/brightness square, hue strip,
  // R/G/B sliders and a hex field. Works on HSV internally so the hue
  // survives when the color passes through gray.
  let { value = $bindable("#6750A4"), swatches = [] }: { value?: string; swatches?: string[] } = $props();

  function hexToRgb(h: string): [number, number, number] {
    const n = parseInt(h.slice(1), 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }
  function rgbToHex(r: number, g: number, b: number) {
    return "#" + [r, g, b].map((v) => Math.round(v).toString(16).padStart(2, "0")).join("").toUpperCase();
  }
  function rgbToHsv(r: number, g: number, b: number): [number, number, number] {
    r /= 255; g /= 255; b /= 255;
    const max = Math.max(r, g, b), min = Math.min(r, g, b), d = max - min;
    let h = 0;
    if (d) {
      if (max === r) h = ((g - b) / d) % 6;
      else if (max === g) h = (b - r) / d + 2;
      else h = (r - g) / d + 4;
      h = (h * 60 + 360) % 360;
    }
    return [h, max ? d / max : 0, max];
  }
  function hsvToRgb(h: number, s: number, v: number): [number, number, number] {
    const f = (n: number) => {
      const k = (n + h / 60) % 6;
      return (v - v * s * Math.max(0, Math.min(k, 4 - k, 1))) * 255;
    };
    return [f(5), f(3), f(1)];
  }

  const valid = (h: string) => /^#[0-9a-f]{6}$/i.test(h);
  // svelte-ignore state_referenced_locally
  let [h0, s0, v0] = rgbToHsv(...hexToRgb(valid(value) ? value : "#6750A4"));
  let hue = $state(h0);
  let sat = $state(s0);
  let val = $state(v0);
  let hexText = $state(value.toUpperCase());

  const rgbNow = $derived(hsvToRgb(hue, sat, val));

  function commitHsv() {
    value = rgbToHex(...hsvToRgb(hue, sat, val));
    hexText = value;
  }

  function setFromHex(h: string) {
    if (!valid(h)) return;
    const [nh, ns, nv] = rgbToHsv(...hexToRgb(h));
    if (ns > 0 && nv > 0) hue = nh;
    sat = ns;
    val = nv;
    value = h.toUpperCase();
    hexText = value;
  }

  function setChannel(i: number, n: number) {
    const c = [...rgbNow];
    c[i] = Math.max(0, Math.min(255, n || 0));
    setFromHex(rgbToHex(c[0], c[1], c[2]));
  }

  // Follow outside changes (e.g. a swatch picked elsewhere).
  $effect(() => {
    if (valid(value) && value.toUpperCase() !== rgbToHex(...rgbNow)) setFromHex(value);
  });

  function drag(node: HTMLElement, onMove: (x: number, y: number) => void) {
    const move = (e: PointerEvent) => {
      const r = node.getBoundingClientRect();
      onMove(Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)), Math.max(0, Math.min(1, (e.clientY - r.top) / r.height)));
    };
    const down = (e: PointerEvent) => {
      node.setPointerCapture(e.pointerId);
      move(e);
      node.addEventListener("pointermove", move);
    };
    const up = () => node.removeEventListener("pointermove", move);
    node.addEventListener("pointerdown", down);
    node.addEventListener("pointerup", up);
    node.addEventListener("pointercancel", up);
    return { destroy: () => { node.removeEventListener("pointerdown", down); up(); } };
  }

  function keySv(e: KeyboardEvent) {
    const d = e.shiftKey ? 0.1 : 0.02;
    if (e.key === "ArrowLeft") sat = Math.max(0, sat - d);
    else if (e.key === "ArrowRight") sat = Math.min(1, sat + d);
    else if (e.key === "ArrowUp") val = Math.min(1, val + d);
    else if (e.key === "ArrowDown") val = Math.max(0, val - d);
    else return;
    e.preventDefault();
    commitHsv();
  }

  function keyHue(e: KeyboardEvent) {
    const d = e.shiftKey ? 15 : 3;
    if (e.key === "ArrowLeft") hue = (hue - d + 360) % 360;
    else if (e.key === "ArrowRight") hue = (hue + d) % 360;
    else return;
    e.preventDefault();
    commitHsv();
  }

  const channels = ["R", "G", "B"];
</script>

<div class="picker">
  <div
    class="sv"
    style="--hue:{hue}"
    role="slider"
    tabindex="0"
    aria-label="Saturation and brightness"
    aria-valuenow={Math.round(sat * 100)}
    onkeydown={keySv}
    use:drag={(x, y) => { sat = x; val = 1 - y; commitHsv(); }}
  >
    <div class="knob" style="left:{sat * 100}%; top:{(1 - val) * 100}%; background:{value}"></div>
  </div>

  <div
    class="hue"
    role="slider"
    tabindex="0"
    aria-label="Hue"
    aria-valuenow={Math.round(hue)}
    onkeydown={keyHue}
    use:drag={(x) => { hue = Math.min(359.9, x * 360); commitHsv(); }}
  >
    <div class="knob small" style="left:{(hue / 360) * 100}%; background:hsl({hue} 100% 50%)"></div>
  </div>

  <div class="rgb">
    {#each channels as ch, i}
      <div class="chan">
        <span class="lbl">{ch}</span>
        <input
          type="range"
          min="0"
          max="255"
          value={Math.round(rgbNow[i])}
          style="--from:{rgbToHex(...(rgbNow.map((v, j) => (j === i ? 0 : v)) as [number, number, number]))};--to:{rgbToHex(...(rgbNow.map((v, j) => (j === i ? 255 : v)) as [number, number, number]))}"
          oninput={(e) => setChannel(i, Number((e.target as HTMLInputElement).value))}
          aria-label={ch}
        />
        <input
          class="num"
          type="number"
          min="0"
          max="255"
          value={Math.round(rgbNow[i])}
          onchange={(e) => setChannel(i, Number((e.target as HTMLInputElement).value))}
          aria-label="{ch} value"
        />
      </div>
    {/each}
  </div>

  <div class="bottom">
    <div class="preview" style="background:{value}"></div>
    <md-outlined-text-field
      label="Hex"
      value={hexText}
      maxlength="7"
      oninput={(e: Event) => {
        let t = (e.target as HTMLInputElement).value.trim();
        if (!t.startsWith("#")) t = "#" + t;
        hexText = t;
        setFromHex(t);
      }}
    ></md-outlined-text-field>
  </div>

  {#if swatches.length}
    <div class="swatches">
      {#each swatches as c}
        <button class="sw" style="background:{c}" aria-label={c} title={c} onclick={() => setFromHex(c)}></button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 14px;
    width: 100%;
  }
  .sv {
    position: relative;
    height: 180px;
    border-radius: var(--shape-m);
    background:
      linear-gradient(to top, #000, transparent),
      linear-gradient(to right, #fff, hsl(var(--hue) 100% 50%));
    cursor: crosshair;
    touch-action: none;
    outline: none;
  }
  .hue {
    position: relative;
    height: 16px;
    border-radius: 8px;
    background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00);
    cursor: pointer;
    touch-action: none;
    outline: none;
  }
  .sv:focus-visible,
  .hue:focus-visible {
    box-shadow: 0 0 0 3px var(--md-sys-color-primary);
  }
  .knob {
    position: absolute;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 3px solid #fff;
    box-shadow: 0 0 0 1px rgb(0 0 0 / 0.35), 0 1px 4px rgb(0 0 0 / 0.4);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }
  .knob.small {
    top: 50%;
  }
  .rgb {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .chan {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .lbl {
    width: 14px;
    font-weight: 600;
    font-size: 13px;
    color: var(--md-sys-color-on-surface-variant);
  }
  input[type="range"] {
    flex: 1;
    appearance: none;
    height: 12px;
    border-radius: 6px;
    background: linear-gradient(to right, var(--from), var(--to));
    outline: none;
  }
  input[type="range"]::-webkit-slider-thumb {
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    border: 1px solid rgb(0 0 0 / 0.3);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.4);
    cursor: pointer;
  }
  input[type="range"]:focus-visible::-webkit-slider-thumb {
    box-shadow: 0 0 0 3px var(--md-sys-color-primary);
  }
  .num {
    width: 58px;
    height: 32px;
    padding: 0 6px;
    border-radius: var(--shape-s);
    border: 1px solid var(--md-sys-color-outline);
    background: transparent;
    color: var(--md-sys-color-on-surface);
    font: inherit;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .bottom {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .preview {
    width: 56px;
    height: 56px;
    flex-shrink: 0;
    border-radius: var(--shape-m);
    border: 1px solid var(--md-sys-color-outline-variant);
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .sw {
    all: unset;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    cursor: pointer;
    border: 1px solid var(--md-sys-color-outline-variant);
  }
  .sw:focus-visible {
    outline: 3px solid var(--md-sys-color-primary);
    outline-offset: 2px;
  }
</style>
