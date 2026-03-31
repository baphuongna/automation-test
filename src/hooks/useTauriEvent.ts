import { useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { EventName, EventPayloadMap } from "../types";

function isTauriRuntimeAvailable(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function useTauriEvent<TName extends EventName>(
  eventName: TName,
  handler: (payload: EventPayloadMap[TName]) => void
): void {
  const handlerRef = useRef(handler);

  useEffect(() => {
    handlerRef.current = handler;
  }, [handler]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    if (!isTauriRuntimeAvailable()) {
      const handlePreviewEvent = (event: Event): void => {
        const customEvent = event as CustomEvent<EventPayloadMap[TName]>;
        handlerRef.current(customEvent.detail);
      };

      window.addEventListener(eventName, handlePreviewEvent as EventListener);

      return () => {
        window.removeEventListener(eventName, handlePreviewEvent as EventListener);
      };
    }

    let unlisten: UnlistenFn | null = null;
    let isMounted = true;

    const subscribe = async (): Promise<void> => {
      unlisten = await listen<EventPayloadMap[TName]>(eventName, (event) => {
        handlerRef.current(event.payload);
      });

      if (!isMounted) {
        unlisten();
        unlisten = null;
      }
    };

    void subscribe();

    return () => {
      isMounted = false;

      if (unlisten) {
        void unlisten();
      }
    };
  }, [eventName]);
}
