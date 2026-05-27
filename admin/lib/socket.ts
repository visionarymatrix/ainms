import { io, Socket } from "socket.io-client";
import { useEffect, useRef, useCallback, useState } from "react";

const SOCKET_URL = process.env.NEXT_PUBLIC_SOCKET_URL || "http://localhost:8440";

export interface DeviceStatusEvent {
  device_id: string;
  status: "online" | "offline";
  company_id?: string;
}

export interface ScreenshotReadyEvent {
  request_id: string;
  device_id: string;
  status: string;
  image_path: string;
}

export function useSocket(token: string | null) {
  const socketRef = useRef<Socket | null>(null);
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    if (!token) return;

    const socket = io(SOCKET_URL, {
      path: "/socketio/",
      query: { token, type: "admin" },
      transports: ["websocket"],
      reconnection: true,
      reconnectionAttempts: 10,
      reconnectionDelay: 3000,
    });

    socket.on("connect", () => {
      console.log("[Socket.IO] Connected");
      setIsConnected(true);
    });

    socket.on("disconnect", () => {
      console.log("[Socket.IO] Disconnected");
      setIsConnected(false);
    });

    socket.on("connect_error", (err) => {
      console.error("[Socket.IO] Connection error:", err.message);
    });

    socketRef.current = socket;

    return () => {
      socket.disconnect();
      socketRef.current = null;
    };
  }, [token]);

  const on = useCallback(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (event: string, handler: (...args: any[]) => void) => {
      socketRef.current?.on(event, handler);
      return () => {
        socketRef.current?.off(event, handler);
      };
    },
    []
  );

  const emit = useCallback(
    (event: string, data: unknown) => {
      socketRef.current?.emit(event, data);
    },
    []
  );

  return { isConnected, on, emit, socket: socketRef };
}
