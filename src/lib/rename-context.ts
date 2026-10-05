export interface RenameContext {
  requestId: number;
  paths: string[];
  status: 'ready' | 'empty' | 'unavailable' | 'timeout';
  message?: string;
}

// Latest request wins, while execution/rollback owns its immutable input batch.
export class ContextQueue {
  seen = 0;
  pending?: RenameContext;
  receive(context: RenameContext): boolean {
    if (context.requestId <= this.seen) return false;
    this.seen = context.requestId;
    this.pending = context;
    return true;
  }
  take(busy: boolean): RenameContext | undefined {
    if (busy) return undefined;
    const context = this.pending;
    this.pending = undefined;
    return context;
  }
}
