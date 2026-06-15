import os
import json
from interfaces import BaseDocumentExporter

class JSONFileExporter(BaseDocumentExporter):
    def export(self, content: dict, output_path: str) -> None:
        output_dir = os.path.dirname(output_path)
        os.makedirs(output_dir, exist_ok=True)
        
        with open(output_path, "w", encoding="utf-8") as json_file:
            json.dump(content, json_file, indent=2, ensure_ascii=False)
            
        print(f"\x1b[35m[OCR] 💾 Saved structured layout JSON artifact to: {output_path}\x1b[0m")