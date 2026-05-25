import os
import io
from interfaces import BaseDocumentLoader

class InMemoryPDFLoader(BaseDocumentLoader):
    """Loads a PDF target into a sterile in-memory BytesIO pool to prevent disk locking."""
    
    def load(self, source_path: str) -> io.BytesIO:
        if not os.path.exists(source_path):
            raise FileNotFoundError(f"❌ Error: Targeted path does not exist: {source_path}")
            
        print(f"📂 [Memory Loader] Streaming file bytes into RAM: {os.path.basename(source_path)}")
        
        with open(source_path, "rb") as disk_file:
            # Read the bytes into memory immediately
            memory_buffer = io.BytesIO(disk_file.read())
            
        # Reset buffer head pointer to the beginning so consumers can read it cleanly
        memory_buffer.seek(0)
        return memory_buffer