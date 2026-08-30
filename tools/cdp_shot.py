# -*- coding: utf-8 -*-
"""CDP screenshot helper: capture full-page or element screenshots from the
Chrome instance started with --remote-debugging-port=9222."""
import base64
import json
import sys
import time

import requests
import websocket


def send(ws, method, params=None, msg_id=1):
    ws.send(json.dumps({"id": msg_id, "method": method, "params": params or {}}))
    while True:
        msg = json.loads(ws.recv())
        if msg.get("id") == msg_id:
            return msg


def main():
    action = sys.argv[1] if len(sys.argv) > 1 else "shot"
    url = sys.argv[2] if len(sys.argv) > 2 else "https://www.bilibili.com/video/BV166tA6SECF"
    out = sys.argv[3] if len(sys.argv) > 3 else "shots/web_video.png"
    scroll_y = int(sys.argv[4]) if len(sys.argv) > 4 else 0

    targets = requests.get("http://127.0.0.1:9222/json").json()
    page = next((t for t in targets if t.get("type") == "page"), None)
    if page is None:
        print("no page target")
        return
    ws = websocket.create_connection(page["webSocketDebuggerUrl"], timeout=30, origin="http://127.0.0.1:9222")

    # navigate
    send(ws, "Page.navigate", {"url": url})
    time.sleep(6)

    # viewport: wide and tall like a desktop
    send(ws, "Emulation.setDeviceMetricsOverride",
         {"width": 1500, "height": 860, "deviceScaleFactor": 1, "mobile": False})

    if scroll_y:
        send(ws, "Runtime.evaluate",
             {"expression": f"window.scrollTo(0, {scroll_y})"})
        time.sleep(1.5)

    if action == "click":
        # click via coordinates (x, y from argv 5,6) then screenshot
        x, y = int(sys.argv[5]), int(sys.argv[6])
        send(ws, "Input.dispatchMouseEvent",
             {"type": "mousePressed", "x": x, "y": y, "button": "left",
              "clickCount": 1})
        send(ws, "Input.dispatchMouseEvent",
             {"type": "mouseReleased", "x": x, "y": y, "button": "left",
              "clickCount": 1})
        time.sleep(2)

    time.sleep(1)
    result = send(ws, "Page.captureScreenshot", {"format": "png"})
    data = base64.b64decode(result["result"]["data"])
    with open(out, "wb") as f:
        f.write(data)
    print("saved", out)


if __name__ == "__main__":
    main()
