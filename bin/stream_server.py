#!/usr/bin/env python3
"""
bin/stream_server.py — Rehoboam Plane A Simulation Stream Server (Zero-Dependency WebSocket Daemon)

Broadcasts real-time Plane A simulation packets over standard RFC-6455 WebSockets to Plane C viewers.
Runs using 100% Python standard library (asyncio, hashlib, base64, struct) with zero pip dependencies.

Usage:
    python3 bin/stream_server.py [--port 8089] [--interval 1.2]
"""

import sys
import asyncio
import hashlib
import base64
import struct
import json
import random
import time
import argparse

WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

class SimulationState:
    def __init__(self):
        self.tick = 1000
        self.paused = False
        self.interval = 1.2
        self.year = 1066
        self.month = "Jun"
        self.season = "Summer"
        self.weather = "CLEAR_SUNNY"
        self.settlement_tier = "TWO_CASTLES_WAR"
        self.culture = "ANGLO_SHIRE"

        # 100-Person Valley Village
        self.village_name = "Val d'Or"
        self.population = 100
        self.grain_bushels = 2450
        self.treasury_gold = 1240
        self.watermill_active = True
        self.shelter_status = "NORMAL"

        # North Castle: Sapphire Keep
        self.north_name = "Sapphire Keep"
        self.north_wall_integrity = 100
        self.north_garrison = 30
        self.north_banner = "Blue Griffin"

        # South Castle: Crimson Citadel
        self.south_name = "Crimson Citadel"
        self.south_wall_integrity = 100
        self.south_garrison = 30
        self.south_banner = "Red Dragon"

        self.threat_level = "CROSSFIRE_TENSION"

    def step(self):
        self.tick += 1
        month_idx = (self.tick - 1000) % 12
        months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        seasons = ["Winter", "Winter", "Spring", "Spring", "Spring", "Summer", "Summer", "Summer", "Autumn", "Autumn", "Autumn", "Winter"]
        
        self.month = months[month_idx]
        self.season = seasons[month_idx]
        self.year = 1066 + ((self.tick - 1000) // 12)

        # Economic & Demography Dynamics (100 population village)
        if self.season == "Autumn":
            self.grain_bushels += random.randint(40, 90)
        elif self.season == "Winter":
            self.grain_bushels = max(500, self.grain_bushels - random.randint(20, 50))
        else:
            self.grain_bushels = max(500, self.grain_bushels - random.randint(10, 25))

        if random.random() < 0.2:
            self.treasury_gold += random.randint(5, 20)

        weathers = ["CLEAR_SUNNY", "CLEAR_SUNNY", "LIGHT_MIST", "GOLDEN_AFTERNOON", "THUNDER_HAZE"]
        if random.random() < 0.1:
            self.weather = random.choice(weathers)

        return {
            "type": "SIMULATION_TICK",
            "tick": self.tick,
            "calendar": {
                "year": self.year,
                "season": self.season,
                "month": self.month
            },
            "village": {
                "name": self.village_name,
                "population": self.population,
                "grain_stockpile": self.grain_bushels,
                "treasury_gold": self.treasury_gold,
                "watermill_status": "OPERATIONAL" if self.watermill_active else "IDLE",
                "shelter_status": self.shelter_status
            },
            "castle_north": {
                "name": self.north_name,
                "wall_integrity": self.north_wall_integrity,
                "garrison": self.north_garrison,
                "banner": self.north_banner,
                "artillery_readiness": "READY"
            },
            "castle_south": {
                "name": self.south_name,
                "wall_integrity": self.south_wall_integrity,
                "garrison": self.south_garrison,
                "banner": self.south_banner,
                "artillery_readiness": "READY"
            },
            "economy": {
                "population": self.population,
                "grain_stockpile": self.grain_bushels,
                "treasury_gold": self.treasury_gold,
                "soldiers": self.north_garrison + self.south_garrison
            },
            "citadel": {
                "wall_integrity": int((self.north_wall_integrity + self.south_wall_integrity) / 2),
                "tier": self.settlement_tier
            },
            "environment": {
                "weather": self.weather,
                "solar_elevation": 25.0 + math_sin(self.tick * 0.1) * 8.0,
                "threat_level": self.threat_level
            }
        }

def math_sin(x):
    import math
    return math.sin(x)

def build_ws_accept(key: str) -> str:
    hash_obj = hashlib.sha1((key.strip() + WS_GUID).encode("utf-8"))
    return base64.b64encode(hash_obj.digest()).decode("utf-8")

def encode_ws_frame(payload: str) -> bytes:
    msg = payload.encode("utf-8")
    length = len(msg)
    if length <= 125:
        header = struct.pack("!BB", 0x81, length)
    elif length <= 65535:
        header = struct.pack("!BBH", 0x81, 126, length)
    else:
        header = struct.pack("!BBQ", 0x81, 127, length)
    return header + msg

def decode_ws_frame(data: bytes):
    if len(data) < 2:
        return None, b""
    byte1, byte2 = struct.unpack("!BB", data[:2])
    opcode = byte1 & 0x0F
    is_masked = bool(byte2 & 0x80)
    payload_len = byte2 & 0x7F
    offset = 2

    if payload_len == 126:
        if len(data) < 4:
            return None, b""
        payload_len = struct.unpack("!H", data[2:4])[0]
        offset = 4
    elif payload_len == 127:
        if len(data) < 10:
            return None, b""
        payload_len = struct.unpack("!Q", data[2:10])[0]
        offset = 10

    if is_masked:
        if len(data) < offset + 4 + payload_len:
            return None, b""
        masks = data[offset:offset+4]
        raw = data[offset+4:offset+4+payload_len]
        decoded = bytes(b ^ masks[i % 4] for i, b in enumerate(raw))
        rest = data[offset+4+payload_len:]
    else:
        if len(data) < offset + payload_len:
            return None, b""
        decoded = data[offset:offset+payload_len]
        rest = data[offset+payload_len:]

    return (opcode, decoded), rest

class StreamServer:
    def __init__(self, host="0.0.0.0", port=8089, interval=1.2, run_dir=None):
        self.host = host
        self.port = port
        self.interval = interval
        self.run_dir = run_dir
        self.clients = set()
        self.sim = SimulationState()
        self.sim.interval = interval

    async def broadcast(self, packet):
        if not self.clients:
            return
        frame = encode_ws_frame(json.dumps(packet))
        dead = set()
        for writer in self.clients:
            try:
                writer.write(frame)
                await writer.drain()
            except Exception:
                dead.add(writer)
        self.clients -= dead

    async def handle_client(self, reader, writer):
        # 1. HTTP Upgrade Handshake
        buffer = b""
        is_websocket = False
        while True:
            line = await reader.readline()
            if not line:
                writer.close()
                return
            buffer += line
            if line == b"\r\n":
                break

        req = buffer.decode("utf-8", errors="ignore")
        ws_key = None
        for line in req.split("\r\n"):
            if line.lower().startswith("sec-websocket-key:"):
                ws_key = line.split(":", 1)[1].strip()

        if not ws_key:
            writer.write(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
            await writer.drain()
            writer.close()
            return

        # Accept handshake
        accept_token = build_ws_accept(ws_key)
        resp = (
            "HTTP/1.1 101 Switching Protocols\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Accept: {accept_token}\r\n"
            "\r\n"
        )
        writer.write(resp.encode("utf-8"))
        await writer.drain()

        self.clients.add(writer)
        print(f"[STREAM-SERVER] Client connected from {writer.get_extra_info('peername')}. Active clients: {len(self.clients)}")

        # Send initial world state
        welcome_packet = {
            "type": "CONNECTION_ESTABLISHED",
            "server": "Rehoboam Plane A Headless Stream Engine",
            "protocol_version": "1.0-RFC6455",
            "current_state": self.sim.step()
        }
        writer.write(encode_ws_frame(json.dumps(welcome_packet)))
        await writer.drain()

        # Listen for client upstream commands
        rx_buf = b""
        try:
            while True:
                chunk = await reader.read(4096)
                if not chunk:
                    break
                rx_buf += chunk
                while rx_buf:
                    res, rx_buf = decode_ws_frame(rx_buf)
                    if res is None:
                        break
                    opcode, payload = res
                    if opcode == 0x8:  # Close frame
                        return
                    if opcode == 0x9:  # Ping
                        writer.write(struct.pack("!BB", 0x8A, 0))  # Pong
                        await writer.drain()
                        continue
                    if opcode == 0x1:  # Text frame
                        try:
                            msg = json.loads(payload.decode("utf-8"))
                            await self.handle_command(msg)
                        except Exception as e:
                            print(f"[STREAM-SERVER] Error handling command: {e}")
        except Exception:
            pass
        finally:
            self.clients.discard(writer)
            writer.close()
            print(f"[STREAM-SERVER] Client disconnected. Remaining: {len(self.clients)}")

    async def handle_command(self, cmd):
        action = cmd.get("action", "")
        print(f"[STREAM-SERVER] Upstream Command Received: {action}")

        if action == "PAUSE":
            self.sim.paused = True
            await self.broadcast({"type": "SIMULATION_STATUS", "status": "PAUSED"})
        elif action == "RESUME":
            self.sim.paused = False
            await self.broadcast({"type": "SIMULATION_STATUS", "status": "RUNNING"})
        elif action == "SET_SPEED":
            self.interval = max(0.1, float(cmd.get("speed", 1.0)))
            self.sim.interval = self.interval
            await self.broadcast({"type": "SIMULATION_SPEED_CHANGED", "interval_sec": self.interval})
        elif action in ("FIRE_NORTH_BATTERY", "TRIGGER_SIEGE"):
            target_x = cmd.get("target_x", random.choice([-4.0, 4.0]))
            await self.broadcast({
                "type": "CROSS_VALLEY_DISCHARGE",
                "battery": "NORTH_SAPPHIRE_KEEP",
                "source": "NORTH",
                "target_castle": "SOUTH_CRIMSON_CITADEL",
                "payload": "BLUE_ENCHANTED_STONE",
                "target": {"x": target_x, "y": 6.8, "z": 24.5}
            })
            await asyncio.sleep(1.0)
            destroyed = random.randint(16, 36)
            self.sim.south_wall_integrity = max(0, self.sim.south_wall_integrity - random.randint(8, 14))
            await self.broadcast({
                "type": "WALL_BREACH_EVENT",
                "castle": "SOUTH",
                "castle_name": "Crimson Citadel",
                "impact_x": str(target_x),
                "impact_y": "6.8",
                "impact_z": "24.5",
                "voxels_destroyed": destroyed,
                "structural_state": "COLLAPSED_RUBBLE",
                "citadel_integrity_pct": self.sim.south_wall_integrity,
                "message": f"Sapphire Keep trebuchet blasted Crimson Citadel south wall ({destroyed} blocks shattered, integrity {self.sim.south_wall_integrity}%)"
            })
        elif action == "FIRE_SOUTH_BATTERY":
            target_x = cmd.get("target_x", random.choice([-4.0, 4.0]))
            await self.broadcast({
                "type": "CROSS_VALLEY_DISCHARGE",
                "battery": "SOUTH_CRIMSON_CITADEL",
                "source": "SOUTH",
                "target_castle": "NORTH_SAPPHIRE_KEEP",
                "payload": "FLAMING_PITCH_STONE",
                "target": {"x": target_x, "y": 6.8, "z": -24.5}
            })
            await asyncio.sleep(1.0)
            destroyed = random.randint(16, 36)
            self.sim.north_wall_integrity = max(0, self.sim.north_wall_integrity - random.randint(8, 14))
            await self.broadcast({
                "type": "WALL_BREACH_EVENT",
                "castle": "NORTH",
                "castle_name": "Sapphire Keep",
                "impact_x": str(target_x),
                "impact_y": "6.8",
                "impact_z": "-24.5",
                "voxels_destroyed": destroyed,
                "structural_state": "COLLAPSED_RUBBLE",
                "citadel_integrity_pct": self.sim.north_wall_integrity,
                "message": f"Crimson Citadel trebuchet blasted Sapphire Keep north wall ({destroyed} blocks shattered, integrity {self.sim.north_wall_integrity}%)"
            })
        elif action == "FIRE_DUAL_BARRAGE":
            self.sim.threat_level = "ACTIVE_CROSSFIRE_DUEL"
            await self.broadcast({
                "type": "DUAL_BARRAGE_DISCHARGE",
                "message": "⚔️ FULL CROSS-VALLEY ARTILLERY DUEL! Both fortresses launch counter-battery volleys over Val d'Or village!",
                "north_target": {"x": 3.0, "y": 6.8, "z": 24.5},
                "south_target": {"x": -3.0, "y": 6.8, "z": -24.5}
            })
            await asyncio.sleep(1.1)
            des_n = random.randint(18, 32)
            des_s = random.randint(18, 32)
            self.sim.north_wall_integrity = max(0, self.sim.north_wall_integrity - random.randint(9, 15))
            self.sim.south_wall_integrity = max(0, self.sim.south_wall_integrity - random.randint(9, 15))
            await self.broadcast({
                "type": "WALL_BREACH_EVENT",
                "castle": "BOTH",
                "north_integrity": self.sim.north_wall_integrity,
                "south_integrity": self.sim.south_wall_integrity,
                "message": f"Simultaneous impacts! North Keep: {self.sim.north_wall_integrity}%, South Citadel: {self.sim.south_wall_integrity}%"
            })
        elif action == "TRIGGER_DRAGON":
            await self.broadcast({
                "type": "DRAGON_COMBAT_ACTION",
                "entity": "Vermithrax",
                "action": "CROSS_VALLEY_SWEEP",
                "altitude_m": "17.5",
                "message": "🐉 Vermithrax soars through the central valley corridor, breathing flame over the battlements!"
            })
        elif action == "CIVILIAN_ALERT":
            if self.sim.shelter_status == "NORMAL":
                self.sim.shelter_status = "SHELTERED"
                msg = "🔔 Village bell tolls! 100 villagers take refuge inside thatched cottages & watermill cellars!"
            else:
                self.sim.shelter_status = "NORMAL"
                msg = "🕊️ All-clear sounded! 100 villagers return to the fields, well, and forge."
            await self.broadcast({
                "type": "CIVILIAN_ALERT_EVENT",
                "shelter_status": self.sim.shelter_status,
                "message": msg
            })
        elif action == "MOCK_STREAM_BURST":
            # Generate high-frequency burst of 15 simulation packets
            for b_i in range(15):
                tick_data = self.sim.step()
                tick_data["stream_burst"] = f"HIGH_THROUGHPUT_{b_i+1}/15"
                await self.broadcast(tick_data)
                await asyncio.sleep(0.04)
        elif action == "STEP":
            tick_data = self.sim.step()
            await self.broadcast(tick_data)
        elif action in ("REBUILD_CITADEL", "REBUILD_BOTH"):
            self.sim.north_wall_integrity = 100
            self.sim.south_wall_integrity = 100
            self.sim.threat_level = "CROSSFIRE_TENSION"
            await self.broadcast({
                "type": "CITADEL_REPAIRED",
                "target": "BOTH_CASTLES",
                "status": "INTEGRITY_100%",
                "north_integrity": 100,
                "south_integrity": 100,
                "message": "Both castle curtains repaired with freshly dressed ashlar stone masonry."
            })

    async def run_simulation_loop(self):
        print(f"[STREAM-SERVER] Simulation Loop Active (Interval: {self.interval}s)")
        step_counter = 0
        while True:
            if not self.sim.paused and self.clients:
                step_counter += 1
                tick_data = self.sim.step()
                await self.broadcast(tick_data)

                # Periodic Two-Castles War & Village Narrative Events
                if step_counter % 4 == 0:
                    narrative_events = [
                        {
                            "type": "VILLAGE_WATERMILL",
                            "message": f"Val d'Or river watermill processed 45 bu flour (Grain: {self.sim.grain_bushels} bu)",
                            "grain": self.sim.grain_bushels
                        },
                        {
                            "type": "VILLAGE_CENSUS",
                            "message": f"Val d'Or demography audit: Exactly {self.sim.population} villagers active in valley",
                            "pop": self.sim.population
                        },
                        {
                            "type": "NORTH_GARRISON_REPORT",
                            "message": f"Sapphire Keep: 30 Blue Knights manning northern curtain wall (Integrity {self.sim.north_wall_integrity}%)",
                            "integrity": self.sim.north_wall_integrity
                        },
                        {
                            "type": "SOUTH_GARRISON_REPORT",
                            "message": f"Crimson Citadel: 30 Red Knights primed southern trebuchets (Integrity {self.sim.south_wall_integrity}%)",
                            "integrity": self.sim.south_wall_integrity
                        },
                        {
                            "type": "CROSSFIRE_TACTICAL_ASSESSMENT",
                            "message": f"Cross-valley ballistic corridor clear; Wind 4.2 knots SSE; Threat: {self.sim.threat_level}"
                        },
                        {
                            "type": "TRADE_CARAVAN",
                            "message": f"Neutral merchant caravan passed over central village timber bridge (+40 gold)",
                            "gold": self.sim.treasury_gold
                        }
                    ]
                    chosen = random.choice(narrative_events)
                    await self.broadcast(chosen)

            await asyncio.sleep(self.interval)

    async def run_real_stream_loop(self):
        import os
        print(f"[STREAM-SERVER] Real Stream Mode Active for: {self.run_dir}")
        cli = "target/debug/rh-cli"
        if os.path.exists("target/release/rh-cli"):
            cli = "target/release/rh-cli"
        
        cmd = [cli, "tail", self.run_dir, "--canonical", "--wait"]
        print(f"[STREAM-SERVER] Spawning real stream tail: {' '.join(cmd)}")
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )

        months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        while True:
            line_bytes = await proc.stdout.readline()
            if not line_bytes:
                break
            line = line_bytes.decode("utf-8", errors="ignore").strip()
            if not line or line.startswith("#rehoboam") or line.startswith("# STATUS"):
                continue

            if line.startswith("STEP_DIGEST"):
                parts = line.split()
                if len(parts) >= 8:
                    tick = int(parts[3])
                    living = int(parts[4])
                    births = int(parts[5])
                    deaths = int(parts[6])
                    state_hash = parts[7]
                    year = tick // 518400
                    month_idx = (tick % 518400) // 43200
                    packet = {
                        "type": "SIMULATION_TICK",
                        "mode": "REAL_PLANE_A_STREAM",
                        "tick": tick,
                        "calendar": {
                            "year": year,
                            "month": months[month_idx % 12],
                            "season": ["Winter", "Spring", "Summer", "Autumn"][(month_idx // 3) % 4]
                        },
                        "village": {
                            "name": os.path.basename(os.path.abspath(self.run_dir)),
                            "population": living,
                            "births": births,
                            "deaths": deaths,
                            "state_hash": state_hash
                        },
                        "raw": line
                    }
                    await self.broadcast(packet)
                    await asyncio.sleep(self.interval)

            elif line.startswith("EVENT"):
                parts = line.split()
                verb = parts[7] if len(parts) > 7 else "UNKNOWN"
                tick = int(parts[3]) if len(parts) > 3 else 0
                packet = {
                    "type": "PLANE_A_EVENT",
                    "verb": verb,
                    "tick": tick,
                    "raw": line,
                    "message": f"[Plane A Event] {verb} at tick {tick}"
                }
                await self.broadcast(packet)

            elif line.startswith("LIFE_EXIT"):
                parts = line.split()
                tick = int(parts[3]) if len(parts) > 3 else 0
                person_id = parts[4] if len(parts) > 4 else "unknown"
                packet = {
                    "type": "LIFE_EXIT",
                    "tick": tick,
                    "person_id": person_id,
                    "message": f"Person #{person_id} exited simulation at tick {tick}"
                }
                await self.broadcast(packet)

            elif line.startswith("#^ match"):
                pattern = line[2:].strip()
                packet = {
                    "type": "SIFT_MATCH",
                    "pattern": pattern,
                    "message": f"[Plane C Story Sifter] Match: {pattern}"
                }
                await self.broadcast(packet)

    async def start(self):
        server = await asyncio.start_server(self.handle_client, self.host, self.port)
        mode_str = f"REAL STREAM ({self.run_dir})" if self.run_dir else "POTEMKIN MOCK (Synthetic)"
        print(f"[STREAM-SERVER] Plane A WebSocket Broadcast Server listening on ws://{self.host}:{self.port} [{mode_str}]")
        sim_task = self.run_real_stream_loop() if self.run_dir else self.run_simulation_loop()
        await asyncio.gather(
            server.serve_forever(),
            sim_task
        )

def main():
    parser = argparse.ArgumentParser(description="Rehoboam Plane A Stream Server")
    parser.add_argument("--port", type=int, default=8089, help="WebSocket port (default 8089)")
    parser.add_argument("--interval", type=float, default=1.2, help="Tick interval in seconds (default 1.2)")
    parser.add_argument("--run-dir", type=str, default=None, help="Optional path to real Rehoboam run directory to stream live")
    args = parser.parse_args()

    srv = StreamServer(port=args.port, interval=args.interval, run_dir=args.run_dir)
    try:
        asyncio.run(srv.start())
    except KeyboardInterrupt:
        print("\n[STREAM-SERVER] Server shutdown gracefully.")

if __name__ == "__main__":
    main()
