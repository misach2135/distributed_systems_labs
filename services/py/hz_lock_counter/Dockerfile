FROM python:3.13-slim
RUN pip install --no-cache-dir hazelcast-python-client==5.7.0
WORKDIR /app
COPY hazelcast_lock_counter.py .
ENV PYTHONUNBUFFERED=1
ENTRYPOINT ["python", "hazelcast_lock_counter.py"]
CMD ["run"]
