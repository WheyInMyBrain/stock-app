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
        
        reports_dir = data_root / ticker / "nse_annual-reports"
        output_dir = data_root / ticker / "ocr" / "annual-reports"
        
        if not reports_dir.exists():
            print(f"❌ Error: Targeted reports folder missing: {reports_dir}")
            return

        # 🎯 Gather all valid zip and pdf document files inside the directory
        target_files = sorted([
            f for f in os.listdir(reports_dir) 
            if f.lower().endswith('.pdf') or f.lower().endswith('.zip')
        ])
        
        if not target_files:
            print(f"⚠️ No processed targets located inside: {reports_dir}")
            return
            
        print(f"🏁 Located {len(target_files)} potential historical files for {ticker}.")
        print("--------------------------------------------------------")
        
        # Instantiate our core processing engines once outside the loop
        loader = InMemoryPDFLoader()
        processor = DoclingProcessor()
        
        for file_item in target_files:
            # Drop extensions dynamically to name our markdown output
            base_name = os.path.splitext(file_item)[0]
            pdf_path = reports_dir / file_item
            output_path = output_dir / f"{base_name}.md"
            
            # Checkpoint Optimization: Don't process documents twice
            if output_path.exists():
                print(f"⏭️ Skipping {file_item} — Output already exists.")
                continue
                
            print(f"\n🔄 [Processing Next Target] -> {file_item}")
            
            try:
                # Pull file into memory seamlessly (whether zip or pdf)
                pdf_buffer = loader.load(str(pdf_path))
                
                # Calculate total pages using file string bytes
                view = pdf_buffer.getbuffer()
                total_pages = len(re.findall(b'/Type\\s*/Page', view.tobytes()))
                del view
                
                # Run the Docling processor pipeline
                processor.process(pdf_buffer, total_pages=total_pages, output_path=str(output_path))
                print(f"✅ Extracted output successfully captured at: {output_path.name}")
                
            except Exception as item_err:
                print(f"❌ Error extracting target {file_item}: {str(item_err)}")
                # Continue loop to process subsequent documents if one file is corrupt
                continue
                
        print(f"\n🏁 Mass multi-file generation sequence complete for ticker: {ticker}!")
        
    except Exception as e:
        print("\n💥 CRITICAL PIPELINE FAILURE:")
        traceback.print_exc()

if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit("❌ Usage Error: Provide ticker symbol.")
    run_ocr_pipeline(sys.argv[1].upper().strip())