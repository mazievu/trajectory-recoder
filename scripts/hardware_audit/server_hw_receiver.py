import http.server
import socketserver
import json
import os
import sys
from datetime import datetime

PORT = 9999
DATA_FILE = r"D:\tools GTF\trajectory recoder\report\hardware_inventory.json"
CSV_FILE = r"D:\tools GTF\trajectory recoder\report\hardware_inventory.csv"
KNOWLEDGE_CSV = r"D:\OpenClaw-Knowledge\hardware_inventory.csv"

class HardwareHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass # Silent logging

    def do_POST(self):
        if self.path == "/api/hardware":
            content_length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(content_length).decode("utf-8")
            try:
                data = json.loads(body)
                data["received_at"] = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                mname = data.get("MachineName", "UNKNOWN")
                print(f"[+] Nhan duoc cau hinh tu: {mname} ({data.get('CPU')}, RAM: {data.get('RAM_Total_GB')} GB)")

                # Save JSON
                records = {}
                if os.path.exists(DATA_FILE):
                    try:
                        with open(DATA_FILE, "r", encoding="utf-8") as f:
                            records = json.load(f)
                    except Exception:
                        pass
                records[mname] = data
                with open(DATA_FILE, "w", encoding="utf-8") as f:
                    json.dump(records, f, ensure_ascii=False, indent=2)

                # Save CSV
                fieldnames = ["MachineName", "UserName", "IPAddress", "CPU", "Cores", "Threads", "RAM_Total_GB", "RAM_Free_GB", "GPU", "Disks", "OS_Version", "received_at"]
                row = [str(data.get(k, "")).replace('"', '""') for k in fieldnames]
                csv_line = '"' + '","'.join(row) + '"\n'
                
                for fpath in [CSV_FILE, KNOWLEDGE_CSV]:
                    try:
                        write_header = not os.path.exists(fpath)
                        with open(fpath, "a", encoding="utf-8") as f:
                            if write_header:
                                f.write(",".join(fieldnames) + "\n")
                            f.write(csv_line)
                    except Exception:
                        pass

                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b'{"status":"OK"}')
            except Exception as e:
                self.send_response(400)
                self.end_headers()
                self.wfile.write(str(e).encode())
        else:
            self.send_response(404)
            self.end_headers()

def main():
    print(f"[*] Mini Hardware Receiver dang lang nghe tai port {PORT}...")
    print(f"[*] Endpoint: http://192.168.1.24:{PORT}/api/hardware")
    with socketserver.TCPServer(("0.0.0.0", PORT), HardwareHandler) as httpd:
        httpd.serve_forever()

if __name__ == "__main__":
    main()
