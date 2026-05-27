# Use a clean, official slim Python image as our runtime foundation
FROM python:3.12-slim

# Set system-level environment variables
ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    HF_HOME=/root/.cache/huggingface

# Install system dependencies required for image parsing and PDF layouts
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    libgl1-mesa-glx \
    libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

# Establish our internal working directory
WORKDIR /app

# Leverage Docker's layer caching for python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir --upgrade pip && \
    pip install --no-cache-dir -r requirements.txt

# We execute a quick dummy initialization script during the image build phase.
RUN python -c "from docling.document_converter import DocumentConverter; DocumentConverter()"

# Copy the rest of your stock application directory code over 
COPY . .

# Set default execution command targeting the entrypoint engine script
ENTRYPOINT ["python", "ocr/ocr_engine.py"]