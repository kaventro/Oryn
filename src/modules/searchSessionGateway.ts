// src/modules/searchSessionGateway.ts
/**
 * Adapter for the search-session IPC contract. Keeping Tauri details here
 * lets the coordinator depend on a tiny, testable interface.
 */

export interface SearchQuery {
  path?: string;
  pattern?: string;
  content?: string;
  isRegex?: boolean;
  matchCase?: boolean;
  minSize?: number | null;
  maxSize?: number | null;
  notContaining?: string;
  [key: string]: any;
}

export class TauriSearchSessionGateway {
  public api: () => any;
  public clientId: string;

  constructor(api: () => any) {
    this.api = api;
    this.clientId = globalThis.crypto?.randomUUID?.()
      || `search-client-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  }

  public start(query: SearchQuery): Promise<any> {
    return this.api().searchStart({ ...query, clientId: this.clientId });
  }

  public getPage(sessionId: string, offset: number, limit: number): Promise<any> {
    return this.api().searchPage(sessionId, offset, limit);
  }

  public cancel(): Promise<any> {
    return this.api().cancelSearch(this.clientId);
  }

  public release(sessionId: string): Promise<any> {
    return this.api().releaseSearch(sessionId);
  }
}
