#!/usr/bin/env python3

import socket
import struct


def send_cmd(sock, cmd):
    """Send a length-prefixed command and return the response."""
    payload = cmd.encode()
    sock.send(struct.pack("!I", len(payload)) + payload)

    # Read response
    len_bytes = b""
    while len(len_bytes) < 4:
        chunk = sock.recv(4 - len(len_bytes))
        if not chunk:
            raise ConnectionError("Connection closed")
        len_bytes += chunk

    length = struct.unpack("!I", len_bytes)[0]
    response = b""
    while len(response) < length:
        chunk = sock.recv(length - len(response))
        if not chunk:
            raise ConnectionError("Connection closed")
        response += chunk

    return response.decode()


def main():
    # Connect to daemon
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5)

    try:
        sock.connect(("127.0.0.1", 8888))

        print("> METRICS")
        print(send_cmd(sock, "METRICS\n"))

        print("> LIST_ROOMS")
        resp = send_cmd(sock, "LIST_ROOMS\n")
        room_count = resp.count("clients\n")
        print(f"Room count: {room_count}")

        print("> CREATE_ROOM foo-bar-room")
        print(send_cmd(sock, "CREATE_ROOM\nfoo-bar-room\n"))

        print("> ROOM_CLIENTS test-room")
        print(send_cmd(sock, "ROOM_CLIENTS\ntest-room\n"))

        print("> DELETE_ROOM foo-bar-room")
        print(send_cmd(sock, "DELETE_ROOM\nfoo-bar-room\n"))

        print("> METRICS")
        print(send_cmd(sock, "METRICS\n"))

    except ConnectionRefusedError:
        print("Error: Cannot connect to daemon. Is it running?")
    except ConnectionError as e:
        print(f"Connection error: {e}")
    except Exception as e:
        print(f"Error: {e}")
        import traceback

        traceback.print_exc()
    finally:
        sock.close()


if __name__ == "__main__":
    main()
