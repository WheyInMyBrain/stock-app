import os
import sys
import re
import traceback  
from pathlib import Path
from loaders import InMemoryPDFLoader
from processors import DoclingProcessor

def run_ocr_pipeline(ticker: str):
    try:
        script_dir = Path(__file__).resolve().parent
        data_root = script_dir.parent / "data"
        
        pdf_path = data_root / ticker / "nse_annual-reports" / "2024-2025.pdf"
        # 🎯 CHANGED: Swapped target destination path to output an .md file
        output_path = data_root / ticker / "ocr" / "annual-reports" / "2024-2025.md"
        
        if not pdf_path.exists():
            print(f"❌ Error: Targeted report file missing: {pdf_path}")
            return

        print("📂 [Memory Loader] Streaming file bytes into RAM...")
        loader = InMemoryPDFLoader()
        pdf_buffer = loader.load(str(pdf_path))
        
        view = pdf_buffer.getbuffer()
        total_pages = len(re.findall(b'/Type\\s*/Page', view.tobytes()))
        del view
        
        print(f"📋 Target: {ticker} | Size: {total_pages} pages")
        print("--------------------------------------------------------")
        
        processor = DoclingProcessor()
        processor.process(pdf_buffer, total_pages=total_pages, output_path=str(output_path))
        
        # 🎯 CHANGED: Fixed completion print text message
        print(f"\n✅ Pipeline Complete! Markdown file generated at: {output_path}")
        
    except Exception as e:
        print("\n💥 CRITICAL PIPELINE FAILURE:")
        traceback.print_exc()

if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit("❌ Usage Error: Provide ticker symbol.")
    run_ocr_pipeline(sys.argv[1].upper().strip())