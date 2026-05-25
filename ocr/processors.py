import os
import io
import json
import logging
from typing import BinaryIO
from interfaces import BaseDocumentProcessor

logging.getLogger("rapidocr").setLevel(logging.ERROR)
logging.getLogger("docling").setLevel(logging.WARNING)
os.environ["DOCLING_DEVICE"] = "cpu"

from docling.datamodel.base_models import InputFormat, DocItemLabel
from docling.datamodel.pipeline_options import PdfPipelineOptions
from docling.document_converter import DocumentConverter, PdfFormatOption, DocumentStream

class DoclingCPUProcessor(BaseDocumentProcessor):
    def __init__(self):
        # 🎯 SWITCH: Initialize standard lightweight PDF options (defaults to RapidOCR)
        pipeline_options = PdfPipelineOptions()
        pipeline_options.do_ocr = True  # Explicitly guarantee OCR is engaged
        
        self.converter = DocumentConverter(
            format_options={
                InputFormat.PDF: PdfFormatOption(
                    pipeline_options=pipeline_options
                )
            }
        )
        print("⚡ [Docling Engine] Lightweight RapidOCR Engine Initialized.")
        
    def process(self, file_stream: BinaryIO, total_pages: int, output_path: str) -> dict:
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        
        structured_data = {"pages": {}, "merged": {"markdown": ""}}
        merged_markdown_accum = []
        
        raw_pdf_bytes = file_stream.read()
        
        for page_num in range(1, total_pages + 1):
            fresh_stream = io.BytesIO(raw_pdf_bytes)
            doc_stream = DocumentStream(name="document.pdf", stream=fresh_stream, format=InputFormat.PDF)
            
            result = self.converter.convert(doc_stream, page_range=(page_num, page_num))
            doc = result.document
            
            page_key = f"page_{page_num}"
            structured_data["pages"][page_key] = {"headers": [], "tables": [], "paragraphs": []}
            
            for item, level in doc.iterate_items():
                if item.label in [DocItemLabel.TITLE, DocItemLabel.SECTION_HEADER]:
                    structured_data["pages"][page_key]["headers"].append(item.text)
                elif item.label == DocItemLabel.TABLE:
                    structured_data["pages"][page_key]["tables"].append(item.export_to_markdown() if hasattr(item, 'export_to_markdown') else item.text)
                elif item.label in [DocItemLabel.PARAGRAPH, DocItemLabel.TEXT, DocItemLabel.LIST_ITEM]:
                    structured_data["pages"][page_key]["paragraphs"].append(item.text)
            
            page_md = doc.export_to_markdown()
            merged_markdown_accum.append(page_md)
            structured_data["merged"]["markdown"] = "\n\n--- \n\n".join(merged_markdown_accum)
            
            with open(output_path, "w", encoding="utf-8") as f:
                json.dump(structured_data, f, indent=2, ensure_ascii=False)
                
            print(f"▓ Page {page_num}/{total_pages} autosaved via RapidOCR.")
            
        return structured_data