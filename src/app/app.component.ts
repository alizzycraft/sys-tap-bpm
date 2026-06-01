import {
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  inject,
  signal,
} from "@angular/core";
import { listen } from "@tauri-apps/api/event";

type DisplayMode = "icon" | "floating" | "tooltip" | "icon-and-floating";

type BpmUpdate = {
  bpm: number | null;
  tapCount: number;
  isActive: boolean;
  displayMode: DisplayMode;
};

const idleUpdate: BpmUpdate = {
  bpm: null,
  tapCount: 0,
  isActive: false,
  displayMode: "icon-and-floating",
};

@Component({
  selector: "app-root",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <main class="meter" [attr.data-state]="stateName()">
      <div class="label">BPM</div>
      <div class="value">{{ bpmText() }}</div>
      <div class="subvalue">{{ tapText() }}</div>
    </main>
  `,
})
export class AppComponent {
  private readonly destroyRef = inject(DestroyRef);

  readonly update = signal<BpmUpdate>(idleUpdate);
  readonly bpmText = computed(() => {
    const bpm = this.update().bpm;
    return bpm === null ? "--" : Math.round(bpm).toString();
  });
  readonly tapText = computed(() => {
    const update = this.update();
    return update.isActive
      ? `${update.tapCount} tap${update.tapCount === 1 ? "" : "s"}`
      : "tap tray icon";
  });
  readonly stateName = computed(() => (this.update().isActive ? "active" : "idle"));

  constructor() {
    listen<BpmUpdate>("bpm-update", (event) => {
      this.update.set(event.payload);
    })
      .then((unsubscribe) => {
        this.destroyRef.onDestroy(unsubscribe);
      })
      .catch((error: unknown) => {
        console.error("Failed to subscribe to BPM updates", error);
      });
  }
}
