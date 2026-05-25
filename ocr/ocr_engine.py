import os
import sys
import re
from pathlib import Path
from loaders import InMemoryPDFLoader
from processors import DoclingCPUProcessor

def run_step_by_step_pipeline(ticker: str):
    try:
        script_dir = Path(__file__).resolve().parent
        data_root = script_dir.parent / "data"
        
        pdf_path = data_root / ticker / "nse_annual-reports" / "2023-2024.pdf"
        output_path = data_root / ticker / "ocr" / "annual-reports" / "2023-2024.json"
        
        if not pdf_path.exists():
            print(f"❌ Error: File not found: {pdf_path}")
            return

        loader = InMemoryPDFLoader()
        pdf_buffer = loader.load(str(pdf_path))
        
        view = pdf_buffer.getbuffer()
        total_pages = len(re.findall(b'/Type\\s*/Page', view.tobytes()))
        del view
        
        print(f"⏳ Processing {total_pages} pages sequentially...")
        
        processor = DoclingCPUProcessor()
        # Hand off execution and save dynamically
        processor.process(pdf_buffer, total_pages=total_pages, output_path=str(output_path))
        
        print(f"\n✅ Pipeline Complete! Output safely written to: {output_path}")
        
    except Exception as e:
        print(f"Error: {str(e)}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit("❌ Usage Error: Provide ticker symbol.")
    run_step_by_step_pipeline(sys.argv[1].upper().strip())