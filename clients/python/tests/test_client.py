"""
Tests for KotaDB Python client.
"""

import pytest
from unittest.mock import Mock, patch
import requests
from datetime import datetime

from kotadb.client import KotaDB
from kotadb.types import Document, SearchResult, QueryResult, CreateDocumentRequest
from kotadb.exceptions import KotaDBError, ConnectionError, NotFoundError, ServerError


class TestKotaDBClient:
    """Test suite for KotaDB client."""

    def test_parse_url_http(self):
        """Test URL parsing for HTTP URLs."""
        with patch.object(KotaDB, '_test_connection'):
            db = KotaDB("http://localhost:8080")
            assert db.base_url == "http://localhost:8080"

    def test_parse_url_kotadb_scheme(self):
        """Test URL parsing for kotadb:// connection strings."""
        with patch.object(KotaDB, '_test_connection'):
            db = KotaDB("kotadb://localhost:8080/myapp")
            assert db.base_url == "http://localhost:8080"

    def test_parse_url_no_protocol(self):
        """Test URL parsing when no protocol is specified."""
        with patch.object(KotaDB, '_test_connection'):
            db = KotaDB("localhost:8080")
            assert db.base_url == "http://localhost:8080"

    def test_parse_url_environment_variable(self):
        """Test URL parsing from environment variable."""
        with patch.dict('os.environ', {'KOTADB_URL': 'http://localhost:9000'}):
            with patch.object(KotaDB, '_test_connection'):
                db = KotaDB()
                assert db.base_url == "http://localhost:9000"

    def test_parse_url_no_url_or_env(self):
        """Test error when no URL provided and no environment variable."""
        with patch.dict('os.environ', {}, clear=True):
            with pytest.raises(ConnectionError, match="No URL provided"):
                KotaDB()

    @patch('requests.Session.get')
    def test_test_connection_success(self, mock_get):
        """Test successful connection test."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_get.return_value = mock_response
        
        # Should not raise exception
        db = KotaDB("http://localhost:8080")
        assert db.base_url == "http://localhost:8080"

    @patch('requests.Session.get')
    def test_test_connection_failure(self, mock_get):
        """Test connection test failure."""
        mock_get.side_effect = requests.RequestException("Connection refused")
        
        with pytest.raises(ConnectionError, match="Failed to connect"):
            KotaDB("http://localhost:8080")

    @patch('kotadb.client.KotaDB._test_connection')
    @patch('requests.Session.request')
    def test_query_success(self, mock_request, mock_test):
        """Test successful query operation."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            'results': [
                {
                    'document': {
                        'id': 'doc1',
                        'path': '/test.md',
                        'title': 'Test Doc',
                        'content': 'Test content',
                        'tags': ['test'],
                        'created_at': '2024-01-01T00:00:00Z',
                        'updated_at': '2024-01-01T00:00:00Z',
                        'size': 100
                    },
                    'score': 0.95,
                    'content_preview': 'Test content...'
                }
            ],
            'total_count': 1,
            'query_time_ms': 50
        }
        mock_request.return_value = mock_response
        
        db = KotaDB("http://localhost:8080")
        result = db.query("test query")
        
        assert isinstance(result, QueryResult)
        assert result.total_count == 1
        assert result.query_time_ms == 50
        assert len(result.results) == 1
        assert result.results[0].document.title == "Test Doc"
        assert result.results[0].score == 0.95

    @patch('kotadb.client.KotaDB._test_connection')
    @patch('requests.Session.request')
    def test_get_document_success(self, mock_request, mock_test):
        """Test successful document retrieval."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            'id': 'doc1',
            'path': '/test.md',
            'title': 'Test Doc',
            'content': 'Test content',
            'tags': ['test'],
            'created_at': '2024-01-01T00:00:00Z',
            'updated_at': '2024-01-01T00:00:00Z',
            'size': 100
        }
        mock_request.return_value = mock_response
        
        db = KotaDB("http://localhost:8080")
        doc = db.get("doc1")
        
        assert isinstance(doc, Document)
        assert doc.id == "doc1"
        assert doc.title == "Test Doc"
        assert doc.content == "Test content"

    @patch('kotadb.client.KotaDB._test_connection')
    @patch('requests.Session.request')
    def test_get_document_not_found(self, mock_request, mock_test):
        """Test document not found error."""
        mock_response = Mock()
        mock_response.status_code = 404
        mock_request.return_value = mock_response
        
        db = KotaDB("http://localhost:8080")
        
        with pytest.raises(NotFoundError):
            db.get("nonexistent")

    @patch('kotadb.client.KotaDB._test_connection')
    @patch('requests.Session.request')
    def test_insert_document_dict(self, mock_request, mock_test):
        """Test document insertion with dictionary."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.json.return_value = {'id': 'new_doc_id'}
        mock_request.return_value = mock_response
        
        db = KotaDB("http://localhost:8080")
        doc_id = db.insert({
            'path': '/new.md',
            'title': 'New Doc',
            'content': 'New content'
        })
        
        assert doc_id == 'new_doc_id'

    @patch('kotadb.client.KotaDB._test_connection')
    def test_insert_document_missing_fields(self, mock_test):
        """Test document insertion with missing required fields."""
        db = KotaDB("http://localhost:8080")
        
        with pytest.raises(ValidationError, match="Required field 'title' missing"):
            db.insert({
                'path': '/new.md',
                'content': 'New content'
                # Missing 'title'
            })

    @patch('kotadb.client.KotaDB._test_connection')
    @patch('requests.Session.request')
    def test_insert_document_request_object(self, mock_request, mock_test):
        """Test document insertion with CreateDocumentRequest object."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.json.return_value = {'id': 'new_doc_id'}
        mock_request.return_value = mock_response
        
        db = KotaDB("http://localhost:8080")
        request = CreateDocumentRequest(
            path='/new.md',
            title='New Doc',
            content='New content',
            tags=['test']
        )
        doc_id = db.insert(request)
        
        assert doc_id == 'new_doc_id'

    @patch('kotadb.client.KotaDB._test_connection')
    @patch('requests.Session.request')
    def test_delete_document_success(self, mock_request, mock_test):
        """Test successful document deletion."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_request.return_value = mock_response
        
        db = KotaDB("http://localhost:8080")
        result = db.delete("doc1")
        
        assert result is True

    @patch('kotadb.client.KotaDB._test_connection')
    @patch('requests.Session.request')
    def test_server_error(self, mock_request, mock_test):
        """Test server error handling."""
        mock_response = Mock()
        mock_response.status_code = 500
        mock_response.json.return_value = {'error': 'Internal server error'}
        mock_response.text = 'Internal server error'
        mock_request.return_value = mock_response
        
        db = KotaDB("http://localhost:8080")
        
        with pytest.raises(ServerError, match="Internal server error"):
            db.get("doc1")

    @patch('kotadb.client.KotaDB._test_connection')
    @patch('requests.Session.request')
    def test_request_exception(self, mock_request, mock_test):
        """Test request exception handling."""
        mock_request.side_effect = requests.RequestException("Network error")
        
        db = KotaDB("http://localhost:8080")
        
        with pytest.raises(ConnectionError, match="Request failed"):
            db.get("doc1")

    @patch('kotadb.client.KotaDB._test_connection')
    def test_context_manager(self, mock_test):
        """Test context manager functionality."""
        with patch.object(KotaDB, 'close') as mock_close:
            with KotaDB("http://localhost:8080") as db:
                assert isinstance(db, KotaDB)
            mock_close.assert_called_once()


class TestDocumentType:
    """Test Document data type."""

    def test_from_dict(self):
        """Test Document creation from dictionary."""
        data = {
            'id': 'doc1',
            'path': '/test.md',
            'title': 'Test Doc',
            'content': 'Test content',
            'tags': ['test'],
            'created_at': '2024-01-01T00:00:00Z',
            'updated_at': '2024-01-01T00:00:00Z',
            'size': 100,
            'metadata': {'author': 'test'}
        }
        
        doc = Document.from_dict(data)
        
        assert doc.id == 'doc1'
        assert doc.title == 'Test Doc'
        assert doc.tags == ['test']
        assert doc.metadata == {'author': 'test'}
        assert isinstance(doc.created_at, datetime)

    def test_to_dict(self):
        """Test Document conversion to dictionary."""
        doc = Document(
            id='doc1',
            path='/test.md',
            title='Test Doc',
            content='Test content',
            tags=['test'],
            created_at=datetime(2024, 1, 1),
            updated_at=datetime(2024, 1, 1),
            size=100,
            metadata={'author': 'test'}
        )
        
        data = doc.to_dict()
        
        assert data['id'] == 'doc1'
        assert data['title'] == 'Test Doc'
        assert data['tags'] == ['test']
        assert data['metadata'] == {'author': 'test'}


class TestCreateDocumentRequest:
    """Test CreateDocumentRequest data type."""

    def test_to_dict_minimal(self):
        """Test minimal CreateDocumentRequest conversion."""
        request = CreateDocumentRequest(
            path='/test.md',
            title='Test',
            content='Content'
        )
        
        data = request.to_dict()
        
        assert data == {
            'path': '/test.md',
            'title': 'Test',
            'content': 'Content'
        }

    def test_to_dict_full(self):
        """Test full CreateDocumentRequest conversion."""
        request = CreateDocumentRequest(
            path='/test.md',
            title='Test',
            content='Content',
            tags=['test'],
            metadata={'author': 'test'}
        )
        
        data = request.to_dict()
        
        assert data == {
            'path': '/test.md',
            'title': 'Test',
            'content': 'Content',
            'tags': ['test'],
            'metadata': {'author': 'test'}
        }
