import os
import io
import zipfile
from interfaces import BaseDocumentLoader

class InMemoryPDFLoader(BaseDocumentLoader):
    """Loads a PDF or ZIP target into an in-memory BytesIO pool to prevent disk locking."""
    
    def load(self, source_path: str) -> io.BytesIO:
        if not os.path.exists(source_path):
            raise FileNotFoundError(f"❌ Error: Targeted path does not exist: {source_path}")
            
        filename = os.path.basename(source_path)
        print(f"📂 [Memory Loader] Streaming file bytes into RAM: {filename}")
        
        # 🎯 CASE 1: The file is wrapped inside a ZIP archive
        if source_path.endswith('.zip'):
            with zipfile.ZipFile(source_path, 'r') as zip_ref:
                # Find the internal PDF file item name
                pdf_files = [f for f in zip_ref.namelist() if f.lower().endswith('.pdf')]
                
                if not pdf_files:
                    raise FileNotFoundError(f"❌ Error: No internal .pdf file located inside archive: {filename}")
                
                # Take the first matched PDF asset found in the zip structure
                target_pdf_name = pdf_files[0]
                print(f"📦 [Zip Extractor] Extracting internal file into RAM: {target_pdf_name}")
                
                # Read the uncompressed file stream directly into an independent RAM object
                memory_buffer = io.BytesIO(zip_ref.read(target_pdf_name))
                
        # 🎯 CASE 2: The file is a standard raw PDF document
        else:
            with open(source_path, "rb") as disk_file:
                memory_buffer = io.BytesIO(disk_file.read())
            
        # Reset buffer head pointer so consumer processing blocks can read it starting at index 0
        memory_buffer.seek(0)
        return memory_buffer