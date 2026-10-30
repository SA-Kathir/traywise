import { useEffect, useState, type CSSProperties } from "react";
import { commands, events, type MediaAction, type MediaSession } from "../../bindings";
import { Num, Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import { useNow } from "../../shell/useNow";
import nowPlaying from ".";
import type { TileProps } from "..";

// Solid play and pause, as in the design (Segoe's are outlines).
const PLAY = (
  <svg className="media-glyph" viewBox="0 0 16 16" aria-hidden>
    <path d="M4.5 2.5v11l9-5.5z" />
  </svg>
);
const PAUSE = (
  <svg className="media-glyph" viewBox="0 0 16 16" aria-hidden>
    <rect x="3.5" y="2.5" width="3" height="11" rx="1" />
    <rect x="9.5" y="2.5" width="3" height="11" rx="1" />
  </svg>
);

// Segoe Fluent Icons glyphs.
const NEXT = String.fromCodePoint(0xe893);
const PREVIOUS = String.fromCodePoint(0xe892);
const SHUFFLE = String.fromCodePoint(0xe8b1);
const REPEAT_ALL = String.fromCodePoint(0xe8ee);
const REPEAT_ONE = String.fromCodePoint(0xe8ed);

const useNowPlaying = () => useLive(commands.nowPlayingState, events.nowPlayingChanged);
const control = (s: MediaSession, action: MediaAction) => commands.nowPlayingControl(s.key, action);

/** Home tile: what Windows' current app is playing. 1×1 is play/pause; 2×1 adds the progress; 2×2
 *  the artwork, skip and time; 4×2 large artwork and every control. */
export function Tiles({ size, open }: TileProps) {
  const state = useNowPlaying();
  if (!state) return null;
  const s = state.sessions.find((x) => x.key === state.current) ?? state.sessions[0];
  if (!s || size === "1x1")
    return (
      <Tile
        size={size}
        icon={s?.playing ? PAUSE : s ? PLAY : nowPlaying.icon}
        label={s ? s.title || s.app : "Now playing"}
        detail={!s ? "Nothing playing" : s.playing ? "Playing" : "Paused"}
        on={s?.playing}
        onToggle={s?.can.playPause ? () => control(s, "playPause") : undefined}
        onOpen={open}
      />
    );
  return (
    <div className={`tile media size-${size}`}>
      {size !== "2x1" && <Artwork s={s} />}
      <div className="media-body">
        {size === "4x2" && <h2>Now playing</h2>}
        {size === "2x2" && <span className="media-app">{s.app}</span>}
        <button className="media-text" title={`${s.title}\n${s.artist}`} onClick={open}>
          <span className="tile-label">{s.title || s.app}</span>
          <span className="tile-detail">{size === "4x2" && s.artist ? `${s.artist} · ${s.app}` : s.artist || s.app}</span>
        </button>
        {size === "2x1" ? (
          <div className="media-row">
            <PlayPause s={s} />
            <Progress s={s} />
          </div>
        ) : (
          <>
            <Progress s={s} times={size === "4x2"} />
            <div className="media-row">
              {size === "4x2" && <Button s={s} glyph={PREVIOUS} label="Previous" action="previous" enabled={s.can.previous} />}
              <PlayPause s={s} />
              <Button s={s} glyph={NEXT} label="Next" action="next" enabled={s.can.next} />
              {size === "2x2" && <Elapsed s={s} />}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

/** The page: every app with media, Windows' current one first, each with all its controls. */
export default function Panel() {
  const state = useNowPlaying();
  if (!state) return null;
  if (state.sessions.length === 0) return <p className="hint">Nothing is playing. Start music or a video in any app.</p>;
  const sessions = [...state.sessions].sort((a, b) => Number(b.key === state.current) - Number(a.key === state.current));
  return sessions.map((s) => (
    <div key={s.key} className="media media-page">
      <div className="media-head">
        <Artwork s={s} />
        <div className="media-text">
          <span className="tile-label">{s.title || s.app}</span>
          <span className="tile-detail">{[s.artist, s.album].filter(Boolean).join(" · ")}</span>
          <span className="media-app">{s.app}</span>
        </div>
      </div>
      <Progress s={s} times />
      <div className="media-row">
        {/* Only music apps offer shuffle and repeat to other apps (browsers never do). */}
        {s.can.shuffle && s.shuffle !== null && (
          <Button
            s={s}
            glyph={SHUFFLE}
            label={s.shuffle ? "Shuffle on" : "Shuffle off"}
            action={{ shuffle: !s.shuffle }}
            enabled
            on={s.shuffle}
          />
        )}
        <Button s={s} glyph={PREVIOUS} label="Previous" action="previous" enabled={s.can.previous} />
        <PlayPause s={s} />
        <Button s={s} glyph={NEXT} label="Next" action="next" enabled={s.can.next} />
        {s.can.repeat && s.repeat !== null && (
          <Button
            s={s}
            glyph={s.repeat === "track" ? REPEAT_ONE : REPEAT_ALL}
            label={s.repeat === "track" ? "Repeat this track" : s.repeat === "list" ? "Repeat all" : "Repeat off"}
            // Off, then all, then this track, like most players.
            action={{ repeat: s.repeat === "off" ? "list" : s.repeat === "list" ? "track" : "off" }}
            enabled
            on={s.repeat !== "off"}
          />
        )}
      </div>
      {/* So greyed-out skip buttons don't look broken (YouTube, say, only offers them in a playlist). */}
      {!s.can.next && !s.can.previous && <p className="hint">{s.app} doesn't offer skipping for this right now.</p>}
    </div>
  ));
}

function Artwork({ s }: { s: MediaSession }) {
  // Without cover art, an empty square (the Dot look fills it with dots).
  return s.artwork ? <img className="media-art" src={s.artwork} alt="" /> : <span className="media-art" aria-hidden />;
}

function Button(props: { s: MediaSession; glyph: string; label: string; action: MediaAction; enabled: boolean; on?: boolean }) {
  return (
    <button
      className={`icon-button${props.on ? " on" : ""}`}
      aria-label={props.label}
      title={props.label}
      disabled={!props.enabled}
      onClick={() => control(props.s, props.action)}
    >
      {props.glyph}
    </button>
  );
}

function PlayPause({ s }: { s: MediaSession }) {
  return (
    <button
      className="icon-button media-play"
      aria-label={s.playing ? "Pause" : "Play"}
      title={s.playing ? "Pause" : "Play"}
      disabled={!s.can.playPause}
      onClick={() => control(s, "playPause")}
    >
      {s.playing ? PAUSE : PLAY}
    </button>
  );
}

/** Milliseconds into the track now: the app's last report, moved on by the time since while playing. */
function usePosition(s: MediaSession) {
  const now = useNow(s.playing && s.durationMs > 0);
  const at = s.positionMs + (s.playing ? now - (s.updatedAt ?? now) : 0);
  return Math.min(Math.max(0, at), s.durationMs);
}

/** The track's progress, which seeks when dragged (if the app allows it), with the times under it. */
function Progress({ s, times }: { s: MediaSession; times?: boolean }) {
  const at = usePosition(s);
  // While dragging, and after letting go until the app reports its new position, show where it was dropped.
  const [dragged, setDragged] = useState<number | null>(null);
  useEffect(() => setDragged(null), [s.updatedAt]);
  if (s.durationMs <= 0) return null;
  const shown = dragged ?? at;
  return (
    <div className="media-progress">
      <input
        type="range"
        min={0}
        max={s.durationMs}
        step={1000}
        value={shown}
        disabled={!s.can.seek}
        style={{ "--fill": `${(shown / s.durationMs) * 100}%` } as CSSProperties}
        aria-label="Position"
        onChange={(e) => setDragged(Number(e.target.value))}
        onPointerUp={() => dragged !== null && control(s, { seek: dragged })}
        onKeyUp={() => dragged !== null && control(s, { seek: dragged })}
      />
      {times && (
        <div className="media-times">
          <Num text={clock(shown)} />
          <Num text={clock(s.durationMs)} />
        </div>
      )}
    </div>
  );
}

function Elapsed({ s }: { s: MediaSession }) {
  const at = usePosition(s);
  return s.durationMs > 0 ? (
    <span className="media-elapsed">
      <Num text={clock(at)} />
    </span>
  ) : null;
}

/** Milliseconds as "1:05:09" or "3:41". */
function clock(ms: number) {
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}
