export interface Resolution {
  label: string
  width: number
  height: number
}

export interface PreviewConfig {
  /** Design resolution — used as default when preview opens */
  designResolution: { width: number; height: number }
  /** Preset resolutions for quick switching */
  presets: Resolution[]
}

/** Fallback if converter.json fails to load */
const fallback: PreviewConfig = {
  designResolution: { width: 640, height: 960 },
  presets: [
    { label: '640×960', width: 640, height: 960 },
  ],
}

/** Load config from /converter.json (served by Vite from project root) */
export async function loadConfig(): Promise<PreviewConfig> {
  try {
    const res = await fetch('/converter.json')
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    const json = await res.json()
    return {
      designResolution: json.designResolution ?? fallback.designResolution,
      presets: json.presets ?? fallback.presets,
    }
  } catch {
    console.warn('Failed to load converter.json, using fallback config')
    return fallback
  }
}
