# stock-app/ocr/processors.py
import os
import time
from typing import BinaryIO
from interfaces import BaseDocumentProcessor
from docling.document_converter import DocumentConverter, PdfFormatOption, DocumentStream
from docling.datamodel.pipeline_options import PdfPipelineOptions, AcceleratorDevice, AcceleratorOptions
from docling.datamodel.base_models import InputFormat

class DoclingProcessor(BaseDocumentProcessor):
    def __init__(self):
        # Configure advanced pipeline acceleration layers
        pipeline_options = PdfPipelineOptions()
        
        # This triggers deep structural layout tasks to dynamically seek out a GPU device (CUDA)
        # and fallback safely to the CPU grid if none is located
        pipeline_options.accelerator_options = AcceleratorOptions(
            num_threads=4, # Controls high-performance multithreading boundaries on CPUs
            device=AcceleratorDevice.AUTO # Natively toggles CUDA if active in the container environment
        )
        
        self.converter = DocumentConverter(
            format_options={
                InputFormat.PDF: PdfFormatOption(pipeline_options=pipeline_options)
            }
        )
        print("\x1b[35m[OCR] 🚀 [Docling Engine] Hardware-aware DocumentConverter initialized.\x1b[0m")
        
    def process(self, file_stream: BinaryIO, total_pages: int, output_path: str) -> str:
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        
        doc_stream = DocumentStream(
            name="document.pdf", 
            stream=file_stream, 
            format=InputFormat.PDF
        )
        
        print(f"\x1b[35m[OCR] ⏳ [Docling] Beginning native conversion on {total_pages} pages...\x1b[0m")
        start_time = time.time()
        
        result = self.converter.convert(doc_stream)
        full_markdown = result.document.export_to_markdown()
        
        elapsed = time.time() - start_time
        
        with open(output_path, "w", encoding="utf-8") as f:
            f.write(full_markdown)
            
        print(f"\x1b[35m[OCR] ✅ Processing complete in {elapsed:.1f}s! Saved: {output_path}\x1b[0m")
        return full_markdown