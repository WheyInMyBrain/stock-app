import io
from abc import ABC, abstractmethod
from typing import BinaryIO

class BaseDocumentLoader(ABC):
    @abstractmethod
    def load(self, source_path: str) -> io.BytesIO:
        pass

class BaseDocumentProcessor(ABC):
    @abstractmethod
    def process(self, file_stream: BinaryIO) -> dict:
        pass

class BaseDocumentExporter(ABC):
    @abstractmethod
    def export(self, content: dict, output_path: str) -> None:
        pass