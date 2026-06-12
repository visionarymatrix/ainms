"use client";

import { useSocket } from "@/lib/socket";
import { getToken } from "@/lib/auth/session";

export function useSocketStatus() {
  const token = getToken();
  const { isConnected, on, emit, socket } = useSocket(token);

  return { isConnected, on, emit, socket };
}