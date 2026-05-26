import os
import sys
import time
import threading
from typing import BinaryIO
from interfaces import BaseDocumentProcessor
from docling.document_converter import DocumentConverter, DocumentStream
from docling.datamodel.base_models import InputFormat

class DoclingProcessor(BaseDocumentProcessor):
    def __init__(self):
        self.converter = DocumentConverter()
        print("🚀 [Docling Engine] Standard DocumentConverter initialized.")
        
    def process(self, file_stream: BinaryIO, total_pages: int, output_path: str) -> str:
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        
        doc_stream = DocumentStream(
            name="2024-2025.pdf", 
            stream=file_stream, 
            format=InputFormat.PDF
        )
        
        # Dynamic Terminal Loading Animation Routine
        done = False
        def animate_loader():
            animation_frames = ["▖", "▘", "▝", "▗"]
            idx = 0
            start_time = time.time()
            while not done:
                elapsed = time.time() - start_time
                sys.stdout.write(f"\r⏳ [Docling] Parsing {total_pages} pages natively... {animation_frames[idx]} ({elapsed:.1f}s elapsed)")
                sys.stdout.flush()
                idx = (idx + 1) % len(animation_frames)
                time.sleep(0.15)
            sys.stdout.write("\r" + " " * 70 + "\r")
            sys.stdout.flush()

        loader_thread = threading.Thread(target=animate_loader)
        loader_thread.start()
        
        try:
            result = self.converter.convert(doc_stream)
            full_markdown = result.document.export_to_markdown()
        finally:
            done = True
            loader_thread.join()
        
        # 🎯 CHANGED: Writing the raw markdown block string natively with no formatting modifications
        with open(output_path, "w", encoding="utf-8") as f:
            f.write(full_markdown)
            
        print(f"✅ Processing complete! Raw markdown file saved inside: {output_path}")
        return full_markdown