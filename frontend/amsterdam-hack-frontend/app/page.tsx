'use client'

import { useEffect, useState } from "react";

interface DroneInfo {
  score: number,
  timestamp: string,
  uav_type: string,
}

export default function Home() {
  const [info, setInfo] = useState<DroneInfo | null>(null);
  const threshold = 0.7;

  useEffect(() => {
    const ws = new WebSocket("ws://localhost:3002/ws");

    ws.onopen = () => console.log("WebSocket opened");

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data !== undefined) {
          setInfo(data);
        }
      } catch (err) {
        console.error("Error parsing Websocket message", err);
      }
    }

    ws.onerror = (error) => console.error("Websocket error", error);

    ws.onclose = () => {
      console.log("WebSocket disconnected, attempting to reconnect...");
      setTimeout(() => { window.location.reload(); }, 3000)
    };

    return () => ws.close(); // Clean up
  }, []);

  return (
      <div className="flex justify-center items-center h-screen">
        {info?.score ?? 0 >= threshold ?
          (
            <div className="flex flex-col gap-5 py-10 px-20 bg-red-600 text-white text-center font-bold rounded-lg shadow">
              <h1 className="text-4xl">DRONE DETECTED</h1>
              <h2 className="">CLASS: {info?.uav_type} </h2>
              <h2 className="">CONFIDENCE SCORE: {Math.round( (info?.score ?? 0) * 100 )}%</h2>
            </div>
          )
          :
          (
            <div className="flex flex-col gap-5 py-10 px-20 bg-gray-500 text-black/50 text-center font-bold rounded-lg shadow">
              <h1 className="text-4xl">NO DRONE DETECTED</h1>
            </div>
          )
        }
      </div>
  );
}
